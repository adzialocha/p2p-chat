use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::str;
use std::time::Duration;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use futures::{Async, Future, Poll, Stream, try_ready};
use tokio::timer::Interval;
use tokio_core::reactor::Handle;
use trust_dns::op::{Message, MessageType, Query};
use trust_dns::rr::{rdata, Name, RData, Record, RecordType};
use trust_dns_proto::multicast::{MdnsQueryType, MdnsStream};
use trust_dns_proto::xfer::SerialMessage;
use trust_dns_proto::{BufStreamHandle};

use crate::crypto;

const ANNOUNCE_FREQUENCY: u64 = 1000;

const MDNS_ADDRESS: &str = "224.0.0.251";
const MDNS_PORT: u16 = 5353;

const NAME_SUFFIX: &str = "chat.local";

pub struct DiscoveryStream {
    multicast_addr: SocketAddr,
    name: Name,
    peer: DiscoveryPeer,
    sender: BufStreamHandle,
    stream: MdnsStream,
}

impl DiscoveryStream {
    pub fn new(
        handle: Handle,
        discovery_key: &[u8],
        port: u16,
    ) -> impl Future<Item=Self, Error=io::Error> {
        // Shorten and convert hash to 40 hex chars
        let discovery_key_hex = hex::encode(discovery_key);
        let discovery_key = discovery_key_hex[..40].to_string();

        // Set DNS name to identify what we are interested in
        let name = Name::from_ascii(&format!("{}.{}", discovery_key, NAME_SUFFIX)).unwrap();

        // Generate individual token to identify ourselves
        let token = crypto::generate_random_token();

        // Define own peer node for discovery
        let peer = DiscoveryPeer {
            addr: Ipv4Addr::UNSPECIFIED,
            port,
            token,
        };

        // Set multicast address and port
        let multicast_addr = SocketAddr::new(MDNS_ADDRESS.parse().unwrap(), MDNS_PORT);

        // Wrap mDNS stream
        let (stream_future, sender) = MdnsStream::new(
            multicast_addr,
            MdnsQueryType::OneShotJoin,
            Some(1),
            None,
            None,
        );

        stream_future.map(move |stream| {
            let discovery_stream = Self {
                multicast_addr,
                name,
                peer,
                sender,
                stream,
            };

            // Start finding peers
            handle.spawn(discovery_stream.announce(ANNOUNCE_FREQUENCY));

            // ... and return a stream of them
            discovery_stream
        })
    }

    fn announce(&self, frequency: u64) -> impl Future<Item=(), Error=()> {
        let addr_clone = self.multicast_addr.clone();
        let sender_clone = self.sender.clone();

        let query = self.create_mdns_question().to_vec().unwrap();

        // Send queries to find new peers every x seconds
        Interval::new_interval(Duration::from_millis(frequency))
            .for_each(move |_| {
                let question_message = SerialMessage::new(query.clone(), addr_clone);

                sender_clone.unbounded_send(question_message).unwrap();

                Ok(())
            })
            .then(|_| Ok(()))
    }

    fn handle_incoming_message(&self, serial_message: SerialMessage) -> Option<DiscoveryPeer> {
        match Message::from_vec(serial_message.bytes()) {
            Ok(message) => {
                // Filter messages looking for same name
                if !message.queries().iter().any(|q| q.name().eq_case(&self.name)) {
                    return None;
                }

                match message.message_type() {
                    MessageType::Query => {
                        let answer_message = SerialMessage::new(
                            self.create_mdns_answer().to_vec().unwrap(),
                            self.multicast_addr
                        );

                        // Respond with answer to query
                        self.sender.unbounded_send(answer_message).unwrap();

                        None
                    }
                    MessageType::Response => {
                        // Check if we got response with required fields
                        match DiscoveryPeer::from_message(&message) {
                            Some(interested_peer) => {
                                // Make sure this is not our response
                                if interested_peer.token != self.peer.token {
                                    Some(interested_peer)
                                } else {
                                    None
                                }
                            }
                            None => None,
                        }
                    }
                }
            },
            Err(_) => None,
        }
    }

    fn create_mdns_question(&self) -> Message {
        let mut message = Message::new();

        let mut query = Query::new();
        query.set_query_type(RecordType::TXT);
        query.set_name(self.name.clone());

        message.add_query(query);

        message
    }

    fn create_mdns_answer(&self) -> Message {
        let mut message = self.create_mdns_question();
        message.set_message_type(MessageType::Response);

        let txt_data = vec![
            format!("token={}", self.peer.token()),
            format!("peers={}", self.peer.encode_peers_field()),
        ];

        let mut record = Record::new();
        record.set_name(self.name.clone());
        record.set_record_type(RecordType::TXT);
        record.set_rdata(RData::TXT(rdata::txt::TXT::new(txt_data)));

        message.add_answer(record);

        message
    }
}

impl Stream for DiscoveryStream {
    type Item = DiscoveryPeer;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match try_ready!(self.stream.poll().map_err(io::Error::from)) {
            Some(message) => {
                match self.handle_incoming_message(message) {
                    Some(peer) => Ok(Async::Ready(Some(peer))),
                    None => Ok(Async::NotReady),
                }
            },
            None => Ok(Async::NotReady),
        }
    }
}

pub struct DiscoveryPeer {
    addr: Ipv4Addr,
    port: u16,
    token: String,
}

impl DiscoveryPeer {
    pub fn addr(&self) -> Ipv4Addr {
        self.addr
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn token(&self) -> String {
        self.token.clone()
    }

    fn from_message(message: &Message) -> Option<DiscoveryPeer> {
        // Check TXT records of message for needed fields
        message.answers().iter().find_map(|rr| {
            if let RData::TXT(ref rdata) = *rr.rdata() {
                // Append only "token" and "peers" fields
                let fields: Vec<Vec<&str>> = rdata
                    .iter()
                    .map(|d| str::from_utf8(d).unwrap())
                    .map(|s| s.splitn(2, '=').collect())
                    .filter_map(|t: Vec<&str>| {
                        if t.len() == 2 && (t[0] == "token" || t[0] == "peers") {
                            Some(t)
                        } else {
                            None
                        }
                    })
                    .collect();

                // Both "token" and "peers" should be given
                if fields.len() != 2 {
                    return None;
                }

                let mut map: HashMap<String, String> = HashMap::with_capacity(2);

                for field in fields {
                    map.insert(String::from(field[0]), String::from(field[1]));
                }

                let token = map["token"].clone();
                let peers = map["peers"].clone();

                let (addr, port) = DiscoveryPeer::decode_peers_field(&peers);

                Some(DiscoveryPeer { port, addr, token })
            } else {
                None
            }
        })
    }

    fn encode_peers_field(&self) -> String {
        let mut writer = Vec::new();

        for octet in self.addr().octets().iter() {
            writer.write_u8(*octet).unwrap();
        }

        writer.write_u16::<BigEndian>(self.port()).unwrap();

        base64::encode(&writer)
    }

    fn decode_peers_field(data: &str) -> (Ipv4Addr, u16) {
        let mut reader = io::Cursor::new(base64::decode(data).unwrap());

        let addr = Ipv4Addr::new(
            reader.read_u8().unwrap(),
            reader.read_u8().unwrap(),
            reader.read_u8().unwrap(),
            reader.read_u8().unwrap(),
        );

        let port = reader.read_u16::<BigEndian>().unwrap();

        (addr, port)
    }
}

#[cfg(test)]
mod discovery {
    use super::*;

    #[test]
    fn get() {
        assert_eq!(2, 2);
    }
}
