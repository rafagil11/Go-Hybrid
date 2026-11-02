//! protocol.rs — Global Protocol Projection for Deadlock-Free Communication.

use std::collections::HashMap;

/// A simple representation of a type for protocol payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtoType {
    Int,
    String,
    Bool,
    Unit,
}

/// Global protocol operations representing the "source of truth" for communication.
#[derive(Debug, Clone)]
pub enum GlobalOp {
    /// Role A sends a message with label and payload to Role B.
    Transfer {
        from: String,
        to: String,
        label: String,
        payload: ProtoType,
    },
    /// Role A makes a choice that Role B must follow.
    Choice {
        from: String,
        to: String,
        branches: HashMap<String, Vec<GlobalOp>>,
    },
    /// A block of operations that can repeat.
    Loop {
        body: Vec<GlobalOp>,
    },
}

/// Local projection of a global protocol for a specific role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalOp {
    /// Send a message to a peer.
    Send { to: String, label: String, payload: ProtoType, next: Box<LocalOp> },
    /// Receive a message from a peer.
    Recv { from: String, label: String, payload: ProtoType, next: Box<LocalOp> },
    /// Choose one of several paths to send to a peer.
    Select { to: String, branches: HashMap<String, LocalOp> },
    /// Offer several paths for a peer to choose from.
    Offer { from: String, branches: HashMap<String, LocalOp> },
    /// Repeat a behavior.
    Loop { body: Box<LocalOp>, next: Box<LocalOp> },
    /// End of communication for this protocol.
    Terminate,
}

/// A global protocol definition.
pub struct Protocol {
    pub name: String,
    pub roles: Vec<String>,
    pub sequence: Vec<GlobalOp>,
}

impl Protocol {
    pub fn new(name: &str, roles: Vec<&str>) -> Self {
        Self {
            name: name.to_string(),
            roles: roles.into_iter().map(|s| s.to_string()).collect(),
            sequence: Vec::new(),
        }
    }

    pub fn add_op(&mut self, op: GlobalOp) {
        self.sequence.push(op);
    }

    /// Projects the global protocol onto a specific role.
    pub fn project(&self, role: &str) -> LocalOp {
        self.project_sequence(&self.sequence, role)
    }

    fn project_sequence(&self, sequence: &[GlobalOp], role: &str) -> LocalOp {
        if sequence.is_empty() {
            return LocalOp::Terminate;
        }

        let (first, rest) = sequence.split_first().unwrap();
        let next = self.project_sequence(rest, role);

        match first {
            GlobalOp::Transfer { from, to, label, payload } => {
                if from == role {
                    LocalOp::Send {
                        to: to.clone(),
                        label: label.clone(),
                        payload: payload.clone(),
                        next: Box::new(next),
                    }
                } else if to == role {
                    LocalOp::Recv {
                        from: from.clone(),
                        label: label.clone(),
                        payload: payload.clone(),
                        next: Box::new(next),
                    }
                } else {
                    next
                }
            }
            GlobalOp::Choice { from, to, branches } => {
                if from == role {
                    let mut projected_branches = HashMap::new();
                    for (label, ops) in branches {
                        projected_branches.insert(label.clone(), self.project_sequence(ops, role));
                    }
                    LocalOp::Select { to: to.clone(), branches: projected_branches }
                } else if to == role {
                    let mut projected_branches = HashMap::new();
                    for (label, ops) in branches {
                        projected_branches.insert(label.clone(), self.project_sequence(ops, role));
                    }
                    LocalOp::Offer { from: from.clone(), branches: projected_branches }
                } else {
                    // Bystander role projection: must behave consistently across all branches.
                    next
                }
            }
            GlobalOp::Loop { body } => {
                let projected_body = self.project_sequence(body, role);
                if projected_body == LocalOp::Terminate {
                    next
                } else {
                    LocalOp::Loop {
                        body: Box::new(projected_body),
                        next: Box::new(next),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_pong_projection() {
        let mut p = Protocol::new("PingPong", vec!["Client", "Server"]);
        p.add_op(GlobalOp::Transfer {
            from: "Client".to_string(),
            to: "Server".to_string(),
            label: "ping".to_string(),
            payload: ProtoType::Int,
        });
        p.add_op(GlobalOp::Transfer {
            from: "Server".to_string(),
            to: "Client".to_string(),
            label: "pong".to_string(),
            payload: ProtoType::String,
        });

        let client_local = p.project("Client");
        let server_local = p.project("Server");

        if let LocalOp::Send { to, label, next, .. } = client_local {
            assert_eq!(to, "Server");
            assert_eq!(label, "ping");
            if let LocalOp::Recv { from, label, .. } = *next {
                assert_eq!(from, "Server");
                assert_eq!(label, "pong");
            } else {
                panic!("Client second op should be Recv");
            }
        } else {
            panic!("Client first op should be Send");
        }

        if let LocalOp::Recv { from, label, next, .. } = server_local {
            assert_eq!(from, "Client");
            assert_eq!(label, "ping");
            if let LocalOp::Send { to, label, .. } = *next {
                assert_eq!(to, "Client");
                assert_eq!(label, "pong");
            } else {
                panic!("Server second op should be Send");
            }
        } else {
            panic!("Server first op should be Recv");
        }
    }
}
