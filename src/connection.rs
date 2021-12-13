use async_trait::async_trait;
use quinn;
use std::convert::TryFrom;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::{fs, path::PathBuf};
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::rustls::{self, OwnedTrustAnchor};
use tokio_rustls::TlsConnector;

const HTTP_REQ_STREAM_ID: u64 = 4;

#[async_trait]
pub trait ProtocolConnection {
    async fn send(&mut self, buffer: &[u8]);
    async fn recv(&mut self);

    async fn close(&mut self);

    //fn abort();
}

pub struct Connection {
    pub protocol_impl: Box<dyn ProtocolConnection>,
}

impl Connection {
    pub async fn send(&mut self, buffer: &[u8]) {
        self.protocol_impl.send(buffer).await;
    }

    pub fn recv(&mut self) {
        self.protocol_impl.recv();
    }

    pub fn close(&mut self) {
        self.protocol_impl.close();
    }
}

pub struct TcpConnection {
    pub stream: tokio::net::TcpStream,
}

impl TcpConnection {
    pub async fn connect(addr: SocketAddr) -> Option<TcpConnection> {
        match tokio::net::TcpStream::connect(addr).await {
            Ok(conn) => Some(TcpConnection { stream: conn }),
            Err(e) => None,
        }
    }
}

#[async_trait]
impl ProtocolConnection for TcpConnection {
    async fn send(&mut self, buffer: &[u8]) {
        self.stream.write(buffer).await.unwrap();
    }

    async fn recv(&mut self) {
        let mut buffer: [u8; 1024] = [0; 1024];
        self.stream.read(&mut buffer).await.unwrap();
    }

    async fn close(&mut self) {
        self.stream.shutdown().await;
        //.expect("failed to shutdown");
    }
}

pub const ALPN_QUIC_HTTP: &[&[u8]] = &[b"hq-29"];

pub struct QuicConnection {
    pub conn: quinn::Connection,
    pub send: quinn::SendStream,
    pub recv: quinn::RecvStream,
}

impl QuicConnection {
    pub async fn connect(
        addr: SocketAddr,
        local_endpoint: SocketAddr,
        cert_path: PathBuf,
        hostname: String,
    ) -> Option<QuicConnection> {
        let mut roots = rustls::RootCertStore::empty();

        match fs::read(cert_path) {
            Ok(v) => match roots.add(&rustls::Certificate(v)) {
                Ok(_) => {}
                Err(_) => return None,
            },
            Err(e) => return None,
        }

        let mut client_crypto = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        client_crypto.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();

        let mut endpoint = quinn::Endpoint::client(local_endpoint).unwrap();
        endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(client_crypto)));

        match endpoint.connect(addr, &hostname) {
            Ok(v) => match v.await {
                Ok(new_conn) => {
                    let quinn::NewConnection {
                        connection: conn, ..
                    } = new_conn;
                    match conn.open_bi().await {
                        Ok((send, recv)) => {
                            return Some(QuicConnection {
                                conn: conn,
                                send: send,
                                recv: recv,
                            })
                        }
                        Err(e) => return None,
                    }
                }
                Err(e) => return None,
            },
            Err(e) => return None,
        }
    }
}
#[async_trait]
impl ProtocolConnection for QuicConnection {
    async fn send(&mut self, buffer: &[u8]) {
        self.send.write_all(buffer).await;
    }

    async fn recv(&mut self) {
        let mut buffer: [u8; 1024] = [0; 1024];
        let resp = self.recv.read(&mut buffer).await;
    }

    async fn close(&mut self) {
        //self.send.finish().await;
        self.conn.close(0u32.into(), b"done");
    }
}
pub enum TlsTcpConn {
    Client(tokio_rustls::client::TlsStream<tokio::net::TcpStream>),
    Server(tokio_rustls::server::TlsStream<tokio::net::TcpStream>),
}

pub struct TlsTcpConnection {
    //stream: tokio::net::TcpStream,
    pub tls_conn: TlsTcpConn, //tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
}

impl TlsTcpConnection {
    pub async fn connect(addr: SocketAddr, host: String) -> Option<TlsTcpConnection> {
        let mut root_cert_store = rustls::RootCertStore::empty();
        root_cert_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(
            |ta| {
                OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject,
                    ta.spki,
                    ta.name_constraints,
                )
            },
        ));

        let config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth(); // i guess this was previously the default?
        let connector = TlsConnector::from(Arc::new(config));

        let stream = tokio::net::TcpStream::connect(&addr).await.unwrap();
        let domain = rustls::ServerName::try_from(host.as_str())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))
            .unwrap();
        let conn = connector.connect(domain, stream).await.unwrap();
        Some(TlsTcpConnection {
            tls_conn: TlsTcpConn::Client(conn),
        })
    }
}

#[async_trait]
impl ProtocolConnection for TlsTcpConnection {
    async fn send(&mut self, buf: &[u8]) {
        match &mut self.tls_conn {
            TlsTcpConn::Client(conn) => {
                conn.write_all(buf).await;
            }
            TlsTcpConn::Server(conn) => {
                conn.write_all(buf).await;
            }
        }
    }

    async fn recv(&mut self) {
        let mut buffer: [u8; 1024] = [0; 1024];
        match &mut self.tls_conn {
            TlsTcpConn::Client(conn) => {
                conn.read(&mut buffer).await;
            }
            TlsTcpConn::Server(conn) => {
                conn.read(&mut buffer).await;
            }
        }
    }

    async fn close(&mut self) {
        match &mut self.tls_conn {
            TlsTcpConn::Client(conn) => {
                conn.shutdown().await;
            }
            TlsTcpConn::Server(conn) => {
                conn.shutdown().await;
            }
        }
    }
}
