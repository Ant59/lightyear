use crate::protocol::{Direction, Inputs, Message1, MyProtocol, PlayerColor, PlayerPosition};
use crate::shared::{shared_config, shared_movement_behaviour};
use crate::{KEY, PROTOCOL_ID};
use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::time::Duration;

#[derive(Resource, Copy, Clone)]
pub struct MyClientPlugin {
    pub(crate) client_id: ClientId,
    pub(crate) server_port: u16,
}

impl Plugin for MyClientPlugin {
    fn build(&self, app: &mut App) {
        let server_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), self.server_port);
        let auth = Authentication::Manual {
            server_addr,
            client_id: self.client_id,
            private_key: KEY,
            protocol_id: PROTOCOL_ID,
        };
        let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
        let link_conditioner = LinkConditionerConfig {
            incoming_latency: Duration::from_millis(100),
            incoming_jitter: Duration::from_millis(10),
            incoming_loss: 0.03,
        };
        let io = Io::from_config(
            &IoConfig::from_transport(TransportConfig::UdpSocket(addr))
                .with_conditioner(link_conditioner),
        );
        let config = ClientConfig {
            shared: shared_config().clone(),
            input: InputConfig::default(),
            netcode: Default::default(),
            ping: PingConfig::default(),
            sync: SyncConfig::default(),
            prediction: PredictionConfig::default(),
            // we are sending updates every frame (60fps), let's add a delay of 6 network-ticks
            interpolation: InterpolationConfig::default()
                .with_delay(InterpolationDelay::Ratio(2.0)),
        };
        let plugin_config = PluginConfig::new(config, io, MyProtocol::default(), auth);
        app.add_plugins(ClientPlugin::new(plugin_config));
        app.add_plugins(crate::shared::SharedPlugin);
        app.insert_resource(self.clone());
        app.add_systems(Startup, init);
        app.add_systems(
            FixedUpdate,
            buffer_input.in_set(InputSystemSet::BufferInputs),
        );
        app.add_systems(FixedUpdate, movement.in_set(FixedUpdateSet::Main));
        app.add_systems(
            Update,
            (
                receive_message1,
                receive_entity_spawn,
                handle_predicted_spawn,
                handle_interpolated_spawn,
                log_interpolated,
                log_confirmed,
            ),
        );
    }
}

// // Resource to store long-term data for the client
// #[derive(Resource, Default)]
// struct Global {
//     pub client_owned_predicted_entity: Option<Entity>,
// }

// Startup system for the client
pub(crate) fn init(
    mut commands: Commands,
    mut client: ResMut<Client<MyProtocol>>,
    plugin: Res<MyClientPlugin>,
) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(TextBundle::from_section(
        format!("Client {}", plugin.client_id),
        TextStyle {
            font_size: 30.0,
            color: Color::WHITE,
            ..default()
        },
    ));
    client.connect();
    // client.set_base_relative_speed(0.001);
}

// System that reads from peripherals and adds inputs to the buffer
pub(crate) fn buffer_input(mut client: ResMut<Client<MyProtocol>>, keypress: Res<Input<KeyCode>>) {
    let mut input = Direction {
        up: false,
        down: false,
        left: false,
        right: false,
    };
    if keypress.pressed(KeyCode::W) || keypress.pressed(KeyCode::Up) {
        input.up = true;
    }
    if keypress.pressed(KeyCode::S) || keypress.pressed(KeyCode::Down) {
        input.down = true;
    }
    if keypress.pressed(KeyCode::A) || keypress.pressed(KeyCode::Left) {
        input.left = true;
    }
    if keypress.pressed(KeyCode::D) || keypress.pressed(KeyCode::Right) {
        input.right = true;
    }
    if keypress.pressed(KeyCode::Delete) {
        // currently, inputs is an enum and we can only add one input per tick
        return client.add_input(Inputs::Delete);
    }
    if keypress.pressed(KeyCode::Space) {
        return client.add_input(Inputs::Spawn);
    }
    // TODO: should we only send an input if it's not all NIL?
    // info!("Sending input: {:?} on tick: {:?}", &input, client.tick());
    if !input.is_none() {
        client.add_input(Inputs::Direction(input));
    }
}

// The client input only gets applied to predicted entities that we own
// This works because we only predict the user's controlled entity.
// If we were predicting more entities, we would have to only apply movement to the player owned one.
pub(crate) fn movement(
    // TODO: maybe make prediction mode a separate component!!!
    mut position_query: Query<&mut PlayerPosition, With<Predicted>>,
    mut input_reader: EventReader<InputEvent<Inputs>>,
) {
    if PlayerPosition::mode() != ComponentSyncMode::Full {
        return;
    }
    for input in input_reader.read() {
        if let Some(input) = input.input() {
            for mut position in position_query.iter_mut() {
                shared_movement_behaviour(&mut position, input);
            }
        }
    }
}

// System to receive messages on the client
pub(crate) fn receive_message1(mut reader: EventReader<MessageEvent<Message1>>) {
    for event in reader.read() {
        info!("Received message: {:?}", event.message());
    }
}

pub(crate) fn receive_entity_spawn(mut reader: EventReader<EntitySpawnEvent>) {
    for event in reader.read() {
        info!("Received entity spawn: {:?}", event.entity());
    }
}

// When the predicted copy of the client-owned entity is spawned, do stuff
// - assign it a different saturation
// - keep track of it in the Global resource
pub(crate) fn handle_predicted_spawn(mut predicted: Query<&mut PlayerColor, Added<Predicted>>) {
    for mut color in predicted.iter_mut() {
        color.0.set_s(0.2);
    }
}

// When the predicted copy of the client-owned entity is spawned, do stuff
// - assign it a different saturation
// - keep track of it in the Global resource
pub(crate) fn handle_interpolated_spawn(
    mut interpolated: Query<&mut PlayerColor, Added<Interpolated>>,
) {
    for mut color in interpolated.iter_mut() {
        info!("SPAWNED INTERPOLATION");
        color.0.set_s(0.2);
    }
}

pub(crate) fn log_confirmed(client: Res<Client<MyProtocol>>, confirmed: Query<&PlayerPosition>) {
    for pos in confirmed.iter() {
        debug!("confirmed pos: {:?}, client tick: {:?}", pos, client.tick());
    }
}

pub(crate) fn log_interpolated(
    interpolated: Query<(&PlayerPosition, &ConfirmedHistory<PlayerPosition>), With<Interpolated>>,
) {
    for (pos, history) in interpolated.iter() {
        debug!("interpolated pos: {:?}, history: {:?}", pos, history);
    }
}