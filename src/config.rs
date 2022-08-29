use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub upstream_lwd: String,
    pub bind_addr: String,
    pub max_outputs_actions: u32,
    pub exclude_sapling: bool,
    pub exclude_orchard: bool,
}
