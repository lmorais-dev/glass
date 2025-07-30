use crate::message::Message;
use crate::message::types::MessageType;
use crate::server::error::ServerError;
use h3::ext::Protocol;
use h3::server::Connection;
use h3::{ConnectionState, quic};
use h3_webtransport::server::AcceptedBi::BidiStream;
use h3_webtransport::server::WebTransportSession;
use h3_webtransport::stream::{RecvStream, SendStream};
use http::Method;
use std::collections::HashMap;
use std::ops::Add;
use std::ptr::read;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::bytes::Bytes;
use tracing::{debug, error, info};

pub async fn handle_h3(
    mut h3_connection: Connection<h3_quinn::Connection, Bytes>,
) -> Result<(), ServerError> {
    loop {
        // Here we try accepting new requests from the h3 connection.
        // This will block the current task until a new request comes in, a GOAWAY package comes or
        // an error is thrown.
        match h3_connection.accept().await {
            Ok(Some(resolver)) => {
                let (request, stream) = match resolver.resolve_request().await {
                    Ok(request) => request,
                    Err(error) => {
                        debug!(?error, "Failed to resolve the request");
                        return Err(ServerError::Resolver);
                    }
                };

                let extensions = request.extensions();
                let is_web_transport = extensions
                    .get::<Protocol>()
                    .eq(&Some(&Protocol::WEB_TRANSPORT));

                // Only accept CONNECT requests that specify the WebTransport spec.
                match request.method() {
                    &Method::CONNECT if is_web_transport => {
                        info!("Upgrading connection to a WebTransport Session");

                        let session =
                            WebTransportSession::accept(request, stream, h3_connection).await;
                        let session = match session {
                            Ok(session) => session,
                            Err(error) => {
                                debug!(
                                    ?error,
                                    "Failed to upgrade the connection to use WebTransport"
                                );
                                return Err(ServerError::Stream(error));
                            }
                        };

                        tokio::spawn(async move {
                            if let Err(error) = handle_session(session).await {
                                debug!(?error, "Failed to handle WebTransport session");
                            }
                        });

                        return Ok(());
                    }
                    _ => {
                        debug!(method = ?request.method(), ?request, "Unsupported method")
                    }
                }
            }
            Ok(None) => break,
            Err(error) => {
                debug!(?error, "H3 Connection errored");
                break;
            }
        }
    }

    Ok(())
}

async fn handle_session(
    session: WebTransportSession<h3_quinn::Connection, Bytes>,
) -> Result<(), ServerError> {
    let session_id = session.session_id();

    // Listen for client stream requests by default all streams are bidirectional even if
    // the operation is unary.
    //
    // Streams are always initiated by the client itself.
    loop {
        let bidi_stream = session.accept_bi().await;
        if let Some(BidiStream(_, stream)) = bidi_stream? {
            let (mut send, mut recv) = quic::BidiStream::split(stream);
            tokio::spawn(async move {
                loop {
                    let message = match read_message(&mut recv).await {
                        Ok(message) => message,
                        Err(error) => {
                            debug!(?error, "Failed to read a message");
                            break;
                        }
                    };
                    debug!(?message, "Received a message from the client");

                    let message_content = match String::from_utf8(message.payload) {
                        Ok(message_content) => message_content,
                        Err(error) => {
                            error!(?error, "Failed to convert the message content to a string");
                            break;
                        }
                    };

                    let response_content = format!("Hello, {message_content}!");
                    let response_message = Message {
                        id: ulid::Ulid::new().0,
                        message_type: MessageType::DataStream,
                        metadata: HashMap::new(),
                        payload: response_content.into_bytes().to_vec(),
                    };

                    match write_message(response_message, &mut send).await {
                        Ok(_) => (),
                        Err(error) => {
                            error!(?error, "Failed to write a message");
                        }
                    };
                }
            });
        }
    }
}

async fn read_message(
    recv: &mut RecvStream<h3_quinn::RecvStream, Bytes>,
) -> Result<Message, ServerError> {
    let mut message_len_buffer = [0u8; 8];
    match recv.read_exact(&mut message_len_buffer).await {
        Ok(_) => (),
        Err(error) => {
            error!(?error, "Failed to read the message length");
        }
    };

    let message_len = u64::from_be_bytes(message_len_buffer);
    let mut message_buffer = Vec::with_capacity(message_len as usize);

    let mut total_bytes_read = 0usize;

    loop {
        if total_bytes_read >= message_len as usize {
            break;
        }

        let mut temp_buffer = [0u8; 128];
        let read_buf = match recv.read(&mut temp_buffer).await {
            Ok(read_count) => read_count,
            Err(error) => {
                debug!(?error, "Failed to read from the stream");
                break;
            }
        };

        total_bytes_read += read_buf;
        message_buffer.extend_from_slice(&temp_buffer[..read_buf]);
    }

    let message = match ciborium::de::from_reader(message_buffer.as_slice()) {
        Ok(message) => message,
        Err(error) => {
            error!(?error, "Failed to deserialize the message");
            return Err(ServerError::Decoding(error));
        }
    };

    Ok(message)
}

async fn write_message(
    message: Message,
    send: &mut SendStream<h3_quinn::SendStream<Bytes>, Bytes>,
) -> Result<(), ServerError> {
    let mut message_buffer = Vec::new();
    
    match ciborium::ser::into_writer(&message, &mut message_buffer) {
        Ok(_) => (),
        Err(error) => {
            error!(?error, "Failed to serialize a message");
            return Err(ServerError::Encoding(error));
        }
    };
    
    let message_len = message_buffer.len() as u64;
    let message_len_buffer = message_len.to_be_bytes();
    
    let mut send_buffer = Vec::new();
    send_buffer.extend_from_slice(&message_len_buffer);
    send_buffer.extend_from_slice(&message_buffer);
    
    let mut send_buffer = Bytes::from(send_buffer);
    
    match send.write_all_buf(&mut send_buffer).await {
        Ok(_) => Ok(()),
        Err(error) => {
            error!(?error, "Failed to send data to the sender stream");
            Err(ServerError::Sender)
        }
    }
}
