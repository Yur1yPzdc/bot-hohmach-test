use std::collections::HashMap;
#[allow(dead_code)]

use std::env;
use std::path::Path;
use dotenv::dotenv;

use serenity::all::ChannelId;
use serenity::async_trait;
use serenity::model::{channel::Message, gateway::Ready};
use serenity::prelude::*;
use serenity::Result as SerenityResult;

// Event related imports to detect track creation failures.
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use songbird::SerenityInit;

const LOG_CHANNEL: ChannelId = ChannelId::new(1385405814440988694);

struct Handler;

#[async_trait]
impl EventHandler for Handler {

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            check_msg(LOG_CHANNEL.say(&ctx.http, "Pong!").await);
        } else if msg.content.to_lowercase() == "бот я призываю тебя" {
            joinvc(&ctx, msg).await;
        } else if msg.content.to_lowercase() == "бот ты свободен" {
            leavevc(&ctx, msg).await;
        } else if msg.content.to_lowercase().contains("бот вруби") {
            play(&ctx, msg).await;
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client =
        Client::builder(&token, intents)
            .event_handler(Handler)
            .register_songbird()
            .await
            .expect("Err creating client");

    tokio::spawn(async move {
        let _ = client
            .start()
            .await
            .map_err(|why| println!("Client ended: {:?}", why));
    });

    let _signal_err = tokio::signal::ctrl_c().await;
    println!("Received Ctrl-C, shutting down.");
}

/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                println!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}

async fn joinvc(ctx: &Context, msg: Message) {
    let (guild_id, channel_id) = {
        let guild = msg.guild(&ctx.cache).unwrap();
        let channel_id = guild
            .voice_states
            .get(&msg.author.id)
            .and_then(|voice_state| voice_state.channel_id);

        (guild.id, channel_id)
    };

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);
            return;
        },
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        // Attach an event handler to see notifications of all track errors.
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
        check_msg(LOG_CHANNEL.say(&ctx.http, "Connected to VC").await);
        }
}

async fn leavevc(ctx: &Context, msg: Message) {
    let guild_id = msg.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Left voice channel").await);
    } else {
        check_msg(msg.reply(ctx, "Not in a voice channel").await);
    }
}

async fn play(ctx: &Context, msg: Message) {
    
    let name_to_path_string: HashMap<&str, &str> = HashMap::from([
        ("Летова", "D:/bot-rs/assets/letov1.mp3"),
        ("генгаозо", "D:/bot-rs/assets/G e n g a o z o -Noize of Nocent-.mp3"),
        ("че-нить пушистое", "D:/bot-rs/assets/fluff.mp3")
    ]);

    let guild_id = msg.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let arg = msg.content.replace("бот вруби ", "");
    let path = match name_to_path_string.get(arg.as_str()) {
        Some(name) => Path::new(*name),
        None => {
            check_msg(msg.channel_id.say(&ctx.http, "Не знаю я такого бля").await);
            return;
        },
    };
    let song = songbird::input::File::new(path);

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        let _ = handler.stop();
        let _ = handler.play_input(song.clone().into());

        check_msg(msg.channel_id.say(&ctx.http, "Playing song").await);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in")
                .await,
        );
    }
}
