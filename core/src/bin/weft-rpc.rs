use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    weft_core::rpc::serve_stdio(None).await
}
