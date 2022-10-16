use async_graphql::{
    http::GraphiQLSource, Context, Object, Result, Schema, SimpleObject, Subscription,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    extract::Extension,
    response::{self, IntoResponse},
    routing::{get, IntoMakeService},
    Router, Server,
};
use futures::{Stream, StreamExt};
use hyper::server::conn::AddrIncoming;
use messenger::messenger_client::MessengerClient;
use messenger::messenger_server::{Messenger, MessengerServer};
use messenger::{ListenRequest, Message, MessageResponse};
use std::{
    collections::HashMap,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub mod messenger {
    tonic::include_proto!("messenger");
}

type MessageResult<T> = Result<Response<T>, Status>;
type ResponseStream = Pin<Box<dyn Stream<Item = Result<Message, Status>> + Send>>;

static COUNTER: AtomicUsize = AtomicUsize::new(1);
fn get_id() -> usize {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Default)]
struct Shared {
    senders: HashMap<usize, mpsc::Sender<Message>>,
}

impl Shared {
    fn new() -> Self {
        Shared {
            senders: HashMap::new(),
        }
    }

    async fn broadcast(&self, msg: Message) {
        // To make our logic simple and consistency, we will broadcast to all
        // users which include msg sender.
        // On frontend, sender will send msg and receive its broadcasted msg
        // and then show his msg on frontend page.
        for (user_id, tx) in &self.senders {
            match tx.send(msg.clone()).await {
                Ok(_) => {}
                Err(_) => {
                    println!("[Broadcast] SendError: to {}, {:?}", user_id, msg)
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct MyMessenger {
    shared: Arc<RwLock<Shared>>,
}

impl MyMessenger {
    fn new(shared: Arc<RwLock<Shared>>) -> Self {
        MyMessenger { shared }
    }
}

#[tonic::async_trait]
impl Messenger for MyMessenger {
    type ListenStream = ResponseStream;

    async fn listen(&self, _request: Request<ListenRequest>) -> MessageResult<Self::ListenStream> {
        println!("HEADERS: {:?}", _request.metadata());

        let (stream_tx, stream_rx) = mpsc::channel(1);

        let user_id = get_id();

        // When connecting, create related sender and reciever
        let (tx, mut rx) = mpsc::channel(1);
        {
            self.shared
                .write()
                .await
                .senders
                .insert(user_id.clone(), tx);
        }

        let shared_clone = self.shared.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match stream_tx.send(Ok(msg)).await {
                    Ok(_) => {}
                    Err(_) => {
                        // If sending failed, then remove the user from shared data
                        println!("[Remote] stream tx sending error. Remote {}", &user_id);
                        shared_clone.write().await.senders.remove(&user_id);
                    }
                }
            }
        });

        let output_stream = ReceiverStream::new(stream_rx);
        Ok(Response::new(Box::pin(output_stream) as Self::ListenStream))
    }

    async fn chat(&self, request: Request<Message>) -> Result<Response<MessageResponse>, Status> {
        self.shared
            .read()
            .await
            .broadcast(request.into_inner())
            .await;

        Ok(Response::new(MessageResponse {}))
    }
}

#[derive(SimpleObject)]
pub struct QMessage {
    text: String,
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn messages(&self) -> Vec<QMessage> {
        vec![]
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn message(&self, _ctx: &Context<'_>, text: String) -> String {
        let mut client = MessengerClient::connect("http://[::1]:50051")
            .await
            .unwrap();

        let request = tonic::Request::new(Message { text: text.clone() });

        let _response = client.chat(request).await.unwrap();

        text.clone()
    }
}

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    async fn messages(&self) -> impl Stream<Item = QMessage> {
        // tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(Duration::from_secs(1)))
        //     .map(move |_| QMessage {
        //         text: "message".to_string(),
        //     })

        let mut client = MessengerClient::connect("http://[::1]:50051")
            .await
            .unwrap();

        client
            .listen(ListenRequest {})
            .await
            .unwrap()
            .into_inner()
            .map(|message| QMessage {
                text: message.unwrap().text,
            })
    }
}

pub type MessagesSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

async fn graphql_handler(
    schema: Extension<MessagesSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn graphiql() -> impl IntoResponse {
    response::Html(
        GraphiQLSource::build()
            .endpoint("http://localhost:8000")
            .subscription_endpoint("ws://localhost:8000/ws")
            .finish(),
    )
}

async fn graphql_server() -> axum::Server<AddrIncoming, IntoMakeService<axum::Router>> {
    let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot).finish();

    let app = Router::new()
        .route("/", get(graphiql).post(graphql_handler))
        .route("/ws", GraphQLSubscription::new(schema.clone()))
        .layer(Extension(schema))
        // .layer(
        //     // see https://docs.rs/tower-http/latest/tower_http/cors/index.html
        //     // for more details
        //     //
        //     // pay attention that for some request types like posting content-type: application/json
        //     // it is required to add ".allow_headers([http::header::CONTENT_TYPE])"
        //     // or see this issue https://github.com/tokio-rs/axum/issues/849
        //     CorsLayer::new()
        //         .allow_origin(any())
        //         .allow_headers(vec![CONTENT_TYPE])
        //         .allow_methods([Method::GET, Method::POST]),
        // )
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    Server::bind(&"0.0.0.0:8000".parse().unwrap()).serve(app.into_make_service())
}

async fn grpc_server() -> impl futures::Future<Output = Result<(), tonic::transport::Error>> {
    let addr = "[::1]:50051".parse().unwrap();

    let shared = Arc::new(RwLock::new(Shared::new()));
    let messenger = MyMessenger::new(shared.clone());

    tonic::transport::Server::builder()
        .add_service(MessengerServer::new(messenger))
        .serve(addr)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = futures::join!(grpc_server().await, graphql_server().await);

    Ok(())
}
