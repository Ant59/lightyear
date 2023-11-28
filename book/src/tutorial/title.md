# Tutorial


This section will teach you how to quickly setup networking in your bevy game using this crate.

You can find an example game in the [examples](https://github.com/cBournhonesque/lightyear/tree/main/tests/examples) folder.

In this tutorial, we will reproduce the [simple box example](https://github.com/cBournhonesque/lightyear/tree/main/tests/examples/simple_box) to demonstrate the features of this crate.

## Setup

First, you will need to define a [Protocol](../concepts/protocol/title.md) for your game.
This is where you define the contract of what is going to be send across the network between your client and server.

A protocol is composed of 
- [Input](../concepts/inputs/title.md): Defines the client's input type, i.e. the different actions that a user can perform
 (e.g. move, jump, shoot, etc.). 
- [Message](../concepts/events/title.md): Defines the message protocol, i.e. the messages that can be exchanged between the client and server. A
  message is any type that is `Send + Sync + 'static` and can be serialized
- [Components](../concepts/replication/title.md): Defines the component protocol, i.e. the list of components that can be replicated between the client and server.

Each of these will be a separate enum.

## Inputs

Let's define our inputs:
```rust
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Direction {
    pub(crate) up: bool,
    pub(crate) down: bool,
    pub(crate) left: bool,
    pub(crate) right: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Inputs {
    Direction(Direction),
    Delete,
    Spawn,
}
impl UserInput for Inputs {}
```

Inputs have to implement the `UserInput` trait, which means that they must be `Send + Sync + 'static` and can be serialized.

## Message

Let's define our message protocol:
```rust
#[derive(Message, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Message1(pub usize);

#[message_protocol(protocol = "MyProtocol")]
pub enum Messages {
    Message1(Message1),
}
```

A message protocol is an enum where each variant is a message that can be sent between the client and server.
Each variant must implement the `Message` trait, and the message protocol must contain the
`#[message_protocol(protocol = "MyProtocol")]` attribute, where `MyProtocol` is the name of the protocol.

## Components

Let's define our components protocol:
```rust
#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerId(ClientId);

#[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Deref, DerefMut, Add, Mul)]
pub struct PlayerPosition(Vec2);

#[derive(Component, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct PlayerColor(pub(crate) Color);

#[component_protocol(protocol = "MyProtocol")]
pub enum Components {
    PlayerId(PlayerId),
    PlayerPosition(PlayerPosition),
    PlayerColor(PlayerColor),
}
```
A component protocol is an enum where each variant is a component that is also serializable and cloneable.
Similarly to message protocol, the enum must contain the `#[component_protocol(protocol = "MyProtocol")]` attribute.

## Channels

We now need to define the [channels](../packet/channels.md) that we will use to send messages between the client and server.
A `Channel` defines how the packets will be sent over the network: reliably? in-order? etc.
```rust
#[derive(Channel)]
pub struct Channel1;
```

We create a channel by simply deriving the `Channel` trait on an empty struct.


## Protocol

We can now create our complete protocol:
```rust
protocolize! {
    Self = MyProtocol,
    Message = Messages,
    Component = Components,
    Input = Inputs,
}

pub(crate) fn protocol() -> MyProtocol {
    let mut protocol = MyProtocol::default();
    protocol.add_channel::<Channel1>(ChannelSettings {
        mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
        direction: ChannelDirection::Bidirectional,
    });
    protocol
}
```

We use the `protocolize!` macro to define our protocol.

We can then define a function that will return an instance of our protocol.
We can use this function to create the same protocol shared between our client and server.

In this function, we can add the various channels that we want to use, and their settings `ChannelSettings`.

## Summary

We now have a complete protocol that defines:
- the data that can be sent between the client and server (inputs, messages, components)
- how the data will be sent (channels)

We can now start building our client and server.