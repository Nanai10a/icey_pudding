use async_trait::async_trait;
use serde_json::{json, Number, Value};
use serenity::builder::CreateMessage;
use serenity::client::{Context, EventHandler};
use serenity::model::channel::Message;
use serenity::model::id::{ChannelId, GuildId, MessageId};

use crate::controllers::SerenityReturnController;
use crate::utils::LetChain;

pub struct Conductor {
    pub contr: SerenityReturnController,
}

#[async_trait]
impl EventHandler for Conductor {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let res = match match self.contr.parse(&msg, &ctx).await {
            Some(r) => r,
            None => return,
        } {
            Ok(mut sv) =>
                msg.channel_id
                    .send_message(&ctx, |cm| {
                        #[allow(clippy::unit_arg)]
                        sv.drain(..)
                            .for_each(|v| cm.add_embed(v).let_(::core::mem::drop))
                            .let_(move |()| {
                                append_message_reference(cm, msg.id, msg.channel_id, msg.guild_id)
                            })
                    })
                    .await,
            Err(e) =>
                msg.channel_id
                    .send_message(&ctx, |cm| {
                        cm.content(format!("```{}```", e)).let_(|cm| {
                            append_message_reference(cm, msg.id, msg.channel_id, msg.guild_id)
                        })
                    })
                    .await,
        };

        #[cfg(not(debug))]
        match res {
            Ok(o) => println!("success: {:?}", o),
            Err(e) => eprintln!("error: {}", e),
        }
    }
}

fn append_message_reference<'a, 'b>(
    raw: &'a mut CreateMessage<'b>,
    id: MessageId,
    channel_id: ChannelId,
    guild_id: Option<GuildId>,
) -> &'a mut CreateMessage<'b> {
    let CreateMessage(map, ..) = raw;

    let mr = json!({
        "message_id": id,
        "channel_id": channel_id,
        "guild_id": match guild_id {
            Some(GuildId(i)) => Value::Number(Number::from(i)),
            None => Value::Null
        },
    });

    map.insert("message_reference", mr);

    raw
}
