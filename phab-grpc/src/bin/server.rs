use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let address = "127.0.0.1:8787".parse().unwrap();

  println!("Server running on {}", address);

  Server::builder()
    .add_service(lib::task_service::new())
    .serve(address)
    .await?;

  return Ok(());
}
