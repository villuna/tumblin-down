use tumblin_down::*;

#[cfg_attr(target_arch = "wasm32", tokio::main(flavor = "current_thread"))]
#[cfg_attr(not(target_arch = "wasm32"), tokio::main)]
async fn main() {
    run().await;
}
