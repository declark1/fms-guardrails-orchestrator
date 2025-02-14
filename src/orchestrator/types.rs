use std::pin::Pin;

use futures::Stream;

pub mod chat_message;
pub use chat_message::{ChatMessage, ChatMessageIterator};
pub mod chunk;
pub mod detection;
pub use chunk::Chunk;
pub use detection::Detection;
pub mod batch_detection_stream;
pub use batch_detection_stream::BatchDetectionStream;

use super::Error;

pub type ChunkerId = String;
pub type DetectorId = String;
pub type Chunks = Vec<Chunk>;
pub type Detections = Vec<Detection>;

pub type BoxStream<T> = Pin<Box<dyn Stream<Item = T> + Send>>;
pub type ChunkStream = BoxStream<Result<Chunk, Error>>;
pub type DetectionStream = BoxStream<Result<(Chunk, Detections), Error>>;
