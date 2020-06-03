use tonic::transport::Server;
use tonic::Request;
use tonic::Response;
use tonic::Status;

mod proto {
  pub mod service {
    tonic::include_proto!("grpc.phab.service");
  }

  pub mod task {
    tonic::include_proto!("grpc.phab.task");
  }
}

use proto::service::task_service_server::TaskService;
use proto::service::task_service_server::TaskServiceServer;
use proto::service::FetchWatchlistInput;
use proto::service::FetchWatchlistOutput;
use proto::task::Task;

#[derive(Default)]
pub struct ImplTaskService {}

#[tonic::async_trait]
impl TaskService for ImplTaskService {
  async fn fetch_watchlist(
    &self,
    req: Request<FetchWatchlistInput>,
  ) -> Result<Response<FetchWatchlistOutput>, Status> {
    return Ok(Response::new(FetchWatchlistOutput {
      tasks: Some(Task {
        id: "wat".to_owned(),
      }),
    }));
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let address = "127.0.0.1:8787".parse().unwrap();

  println!("Server running on {}", address);

  Server::builder()
    .add_service(TaskServiceServer::new(ImplTaskService::default()))
    .serve(address)
    .await?;

  return Ok(());
}
