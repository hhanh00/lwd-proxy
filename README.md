# A lightwalletd proxy

- Proxies requests to another server
- Optionally filters blocks to reduce network bandwidth requirements

# Configuration

```
upstream_lwd = "http://192.168.0.100:9067"
bind_addr = "0.0.0.0:9069"
max_outputs_actions = 50
exclude_sapling = false
exclude_orchard = true
```

| Name  |                                                                     |
| ------|---------------------------------------------------------------------|
| upstream_lwd | URL to the upstreal lightwalletd                             |
| bind_addr | Binding address / port of this server                           |
|  max_outputs_actions | tx that have more than this number of outputs are filtered     |
| exclude_sapling | remove all sapling outputs from tx                                  | 
| exclude_orchard | remove all orchard actions from tx                                  |

Filtered outputs have `cmu` but no `epk` or `ciphertext`. 

**The Client must support receiving outputs without these fields present.**
