use std::net::SocketAddrV4;
use std::str::FromStr;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Server};
use crate::config::Config;
use crate::Error;
use crate::lw_rpc::{Address, AddressList, Balance, BlockId, BlockRange, ChainSpec, CompactBlock, CompactTx, Duration, Empty, Exclude, GetAddressUtxosArg, GetAddressUtxosReply, GetAddressUtxosReplyList, LightdInfo, PingResponse, RawTransaction, SendResponse, TransparentAddressBlockFilter, TreeState, TxFilter};
use crate::lw_rpc::compact_tx_streamer_client::CompactTxStreamerClient;
use crate::lw_rpc::compact_tx_streamer_server::{CompactTxStreamer, CompactTxStreamerServer};

pub async fn connect_lightwalletd(url: &str) -> anyhow::Result<CompactTxStreamerClient<Channel>> {
    let mut channel = tonic::transport::Channel::from_shared(url.to_owned())?;
    if url.starts_with("https") {
        let pem = include_bytes!("ca.pem");
        let ca = Certificate::from_pem(pem);
        let tls = ClientTlsConfig::new().ca_certificate(ca);
        channel = channel.tls_config(tls)?;
    }
    let client = CompactTxStreamerClient::connect(channel).await?;
    Ok(client)
}

struct LWDService {
    config: Config,
    client: Mutex<CompactTxStreamerClient<Channel>>
}

impl LWDService {
    pub async fn new(config: Config) -> crate::Result<Self> {
        let client = connect_lightwalletd(&config.upstream_lwd).await?;
        Ok(LWDService {
            config,
            client: Mutex::new(client),
        })
    }
}

#[tonic::async_trait]
impl CompactTxStreamer for LWDService {
    async fn get_latest_block(&self, request: Request<ChainSpec>) -> Result<Response<BlockId>, Status> {
        let mut client = self.client.lock().await;
        client.get_latest_block(request).await
    }

    async fn get_block(&self, request: Request<BlockId>) -> Result<Response<CompactBlock>, Status> {
        let mut client = self.client.lock().await;
        client.get_block(request).await
    }

    type GetBlockRangeStream = ReceiverStream<Result<CompactBlock, Status>>;

    async fn get_block_range(&self, request: Request<BlockRange>) -> Result<Response<Self::GetBlockRangeStream>, Status> {
        let config = self.config.clone();
        let request = request.into_inner();
        let proxy_request = BlockRange {
            start: request.start,
            end: request.end,
        };
        let lwd_url = self.config.upstream_lwd.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        tokio::spawn(async move {
            let mut client = connect_lightwalletd(&lwd_url).await?;
            let mut blocks = client.get_block_range(Request::new(proxy_request)).await?.into_inner();
            while let Some(mut block) = blocks.message().await? {
                for tx in block.vtx.iter_mut() {
                    if tx.outputs.len() >= config.max_outputs_actions as usize {
                        for co in tx.outputs.iter_mut() {
                            co.epk.clear();
                            co.ciphertext.clear();
                        }
                    }
                    if config.exclude_sapling {
                        tx.outputs = vec![];
                    }
                    if config.exclude_orchard {
                        tx.actions = vec![];
                    }
                }
                let _ = tx.send(Ok(block)).await;
            }
            Ok::<_, Error>(())
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_transaction(&self, request: Request<TxFilter>) -> Result<Response<RawTransaction>, Status> {
        let mut client = self.client.lock().await;
        client.get_transaction(request).await
    }

    async fn send_transaction(&self, request: Request<RawTransaction>) -> Result<Response<SendResponse>, Status> {
        let mut client = self.client.lock().await;
        client.send_transaction(request).await
    }

    type GetTaddressTxidsStream = Streaming<RawTransaction>;

    async fn get_taddress_txids(&self, request: Request<TransparentAddressBlockFilter>) -> Result<Response<Self::GetTaddressTxidsStream>, Status> {
        let mut client = self.client.lock().await;
        client.get_taddress_txids(request).await
    }

    async fn get_taddress_balance(&self, request: Request<AddressList>) -> Result<Response<Balance>, Status> {
        let mut client = self.client.lock().await;
        client.get_taddress_balance(request).await
    }

    async fn get_taddress_balance_stream(&self, _request: Request<Streaming<Address>>) -> Result<Response<Balance>, Status> {
        unimplemented!()
    }

    type GetMempoolTxStream = Streaming<CompactTx>;

    async fn get_mempool_tx(&self, request: Request<Exclude>) -> Result<Response<Self::GetMempoolTxStream>, Status> {
        let mut client = self.client.lock().await;
        client.get_mempool_tx(request).await
    }

    type GetMempoolStreamStream = Streaming<RawTransaction>;

    async fn get_mempool_stream(&self, request: Request<Empty>) -> Result<Response<Self::GetMempoolStreamStream>, Status> {
        let mut client = self.client.lock().await;
        client.get_mempool_stream(request).await
    }

    async fn get_tree_state(&self, request: Request<BlockId>) -> Result<Response<TreeState>, Status> {
        let mut client = self.client.lock().await;
        client.get_tree_state(request).await
    }

    async fn get_address_utxos(&self, request: Request<GetAddressUtxosArg>) -> Result<Response<GetAddressUtxosReplyList>, Status> {
        let mut client = self.client.lock().await;
        client.get_address_utxos(request).await
    }

    type GetAddressUtxosStreamStream = Streaming<GetAddressUtxosReply>;

    async fn get_address_utxos_stream(&self, request: Request<GetAddressUtxosArg>) -> Result<Response<Self::GetAddressUtxosStreamStream>, Status> {
        let mut client = self.client.lock().await;
        client.get_address_utxos_stream(request).await
    }

    async fn get_lightd_info(&self, request: Request<Empty>) -> Result<Response<LightdInfo>, Status> {
        let mut client = self.client.lock().await;
        client.get_lightd_info(request).await
    }

    async fn ping(&self, request: Request<Duration>) -> Result<Response<PingResponse>, Status> {
        let mut client = self.client.lock().await;
        client.ping(request).await
    }
}

const LWD_DESCRIPTOR_SET: &[u8] = include_bytes!("generated/lwd_descriptor.bin");

pub async fn launch(config: Config) -> anyhow::Result<()> {
    let bind_addr = SocketAddrV4::from_str(&config.bind_addr).unwrap().into();
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(LWD_DESCRIPTOR_SET)
        .build().unwrap();
    let handler = LWDService::new(config).await?;
    let service = CompactTxStreamerServer::new(handler);
    Server::builder()
        .add_service(reflection)
        .add_service(service).serve(bind_addr).await?;

    Ok(())
}