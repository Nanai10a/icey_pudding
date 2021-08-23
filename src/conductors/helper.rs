use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

use anyhow::{bail, Result};
use serde_json::{json, Number, Value};
use serenity::builder::CreateEmbed;
use serenity::model::id::{ChannelId, GuildId, MessageId, UserId};
use serenity::model::interactions::application_command::ApplicationCommandInteractionData;
use serenity::utils::Colour;
use uuid::Uuid;

use super::{clapcmd, command_strs, Command, MsgCommand, Response};
use crate::entities::{Content, User};
use crate::repositories::{Comparison, ContentQuery};

pub async fn parse_ia(acid: &ApplicationCommandInteractionData) -> Result<Command> {
    use crate::extract_option;

    let com = match acid.name.as_str() {
        "register" => Command::UserRegister,
        "info" => Command::UserRead,
        "change" => {
            let admin = extract_option!(opt Value::Bool => ref admin in acid)?;
            let sub_admin = extract_option!(opt Value::Bool => ref sub_admin in acid)?;

            Command::UserUpdate(admin.copied(), sub_admin.copied())
        },
        "bookmark" => {
            let id = extract_option!(Value::String => ref id in acid)?;

            Command::Bookmark(Uuid::parse_str(id.as_str())?)
        },
        "delete_me" => Command::UserDelete,
        "post" => {
            let content = extract_option!(Value::String => ref id in acid)?;
            let author = extract_option!(Value::String => ref author in acid)?;

            Command::ContentPost(content.clone(), author.clone())
        },
        "get" => {
            let id = extract_option!(Value::String => ref id in acid)?;

            Command::ContentRead(vec![ContentQuery::Id(Uuid::parse_str(id.as_str())?)])
        },
        "edit" => {
            let id = extract_option!(Value::String => ref id in acid)?;
            let content = extract_option!(Value::String => ref content in acid)?;

            Command::ContentUpdate(Uuid::parse_str(id.as_str())?, content.clone())
        },
        "like" => {
            let id = extract_option!(Value::String => ref id in acid)?;

            Command::Like(Uuid::parse_str(id.as_str())?)
        },
        "pin" => {
            let id = extract_option!(Value::String => ref id in acid)?;

            Command::Pin(Uuid::parse_str(id.as_str())?)
        },
        "remove" => {
            let id = extract_option!(Value::String => ref id in acid)?;

            Command::ContentDelete(Uuid::parse_str(id.as_str())?)
        },
        _ => bail!("unrecognized application_command name."),
    };

    Ok(com)
}

pub async fn parse_msg(msg: &str) -> Result<MsgCommand> {
    let splitted = shell_words::split(msg)?;

    if let Some(n) = splitted.get(0) {
        if n != command_strs::PREFIX {
            bail!("not command, abort.")
        }
    }

    let ams = match clapcmd::create_clap_app().get_matches_from_safe(splitted) {
        Ok(o) => o,
        Err(e) => return Ok(MsgCommand::Showing(e.message)),
    };

    use command_strs::*;

    use super::clapcmd::{extract_clap_arg, extract_clap_sams};

    let cmd = match match ams.subcommand_name() {
        None => bail!("cannot get subcommand."),
        Some(s) => s,
    } {
        register::NAME => Command::UserRegister,
        info::NAME => Command::UserRead,
        change::NAME => {
            let sams = extract_clap_sams(&ams, change::NAME)?;
            let admin_raw = sams.value_of(change::admin::NAME);
            let sub_admin_raw = sams.value_of(change::sub_admin::NAME);

            let admin = match admin_raw.map(|s| bool::from_str(s)) {
                Some(Ok(b)) => Some(b),
                None => None,
                Some(Err(e)) => bail!("{}", e),
            };

            let sub_admin = match sub_admin_raw.map(|s| bool::from_str(s)) {
                Some(Ok(b)) => Some(b),
                None => None,
                Some(Err(e)) => bail!("{}", e),
            };

            Command::UserUpdate(admin, sub_admin)
        },
        bookmark::NAME => {
            let sams = extract_clap_sams(&ams, bookmark::NAME)?;
            let id_raw = extract_clap_arg(sams, bookmark::id::NAME)?;

            let id = Uuid::from_str(id_raw)?;

            Command::Bookmark(id)
        },
        delete_me::NAME => Command::UserDelete,
        post::NAME => {
            let sams = extract_clap_sams(&ams, post::NAME)?;
            let content = extract_clap_arg(sams, post::content::NAME)?;
            let author = extract_clap_arg(sams, post::author::NAME)?;

            Command::ContentPost(content.to_string(), author.to_string())
        },
        get::NAME => {
            let sams = extract_clap_sams(&ams, get::NAME)?;

            let mut queries = vec![];

            if let Ok(o) = extract_clap_arg(sams, get::id::NAME) {
                queries.push(ContentQuery::Id(Uuid::from_str(o)?));
            }
            if let Ok(o) = extract_clap_arg(sams, get::author::NAME) {
                queries.push(ContentQuery::Author(o.to_string()));
            }
            if let Ok(o) = extract_clap_arg(sams, get::posted::NAME) {
                queries.push(ContentQuery::Posted(UserId(o.parse()?)));
            }
            if let Ok(o) = extract_clap_arg(sams, get::content::NAME) {
                queries.push(ContentQuery::Content(o.to_string()));
            }
            if let Ok(o) = extract_clap_arg(sams, get::liked::NAME) {
                let tur = range_syntax_parser(o.to_string())?;
                queries.push(ContentQuery::LikedNum(tur.0, tur.1));
            }
            if let Ok(o) = extract_clap_arg(sams, get::bookmarked::NAME) {
                let tur = range_syntax_parser(o.to_string())?;
                queries.push(ContentQuery::Bookmarked(tur.0, tur.1));
            }
            if let Ok(o) = extract_clap_arg(sams, get::pinned::NAME) {
                let tur = range_syntax_parser(o.to_string())?;
                queries.push(ContentQuery::PinnedNum(tur.0, tur.1));
            }

            Command::ContentRead(queries)
        },
        edit::NAME => {
            let sams = extract_clap_sams(&ams, edit::NAME)?;
            let id_raw = extract_clap_arg(sams, edit::id::NAME)?;
            let content = extract_clap_arg(sams, edit::content::NAME)?;

            let id = Uuid::from_str(id_raw)?;

            Command::ContentUpdate(id, content.to_string())
        },
        like::NAME => {
            let sams = extract_clap_sams(&ams, like::NAME)?;
            let id_raw = extract_clap_arg(sams, like::id::NAME)?;

            let id = Uuid::from_str(id_raw)?;

            Command::Like(id)
        },
        pin::NAME => {
            let sams = extract_clap_sams(&ams, pin::NAME)?;
            let id_raw = extract_clap_arg(sams, pin::id::NAME)?;

            let id = Uuid::from_str(id_raw)?;

            Command::Pin(id)
        },
        remove::NAME => {
            let sams = extract_clap_sams(&ams, remove::NAME)?;
            let id_raw = extract_clap_arg(sams, remove::id::NAME)?;

            let id = Uuid::from_str(id_raw)?;

            Command::ContentDelete(id)
        },
        _ => bail!("unrecognized subcommand."),
    };

    Ok(MsgCommand::Command(cmd))
}

pub fn resp_from_user(
    title: impl Display,
    description: impl Display,
    rgb: (u8, u8, u8),
    User {
        id,
        admin,
        sub_admin,
        posted,
        bookmark,
    }: User,
) -> Response {
    Response {
        title: format!("{}", title),
        rgb,
        description: format!("{}", description),
        fields: vec![
            ("id:".to_string(), format!("{}", id)),
            ("is_admin?".to_string(), format!("{}", admin)),
            ("is_sub_admin?".to_string(), format!("{}", sub_admin)),
            ("posted:".to_string(), format!("{}", posted.len())),
            ("bookmarked:".to_string(), format!("{}", bookmark.len())),
        ],
    }
}

pub fn resp_from_content(
    title: impl Display,
    description: impl Display,
    rgb: (u8, u8, u8),
    Content {
        id,
        content,
        author,
        posted,
        liked,
        bookmarked,
        pinned,
    }: Content,
) -> Response {
    Response {
        title: format!("{}", title),
        rgb,
        description: format!("{}", description),
        fields: vec![
            ("id:".to_string(), format!("{}", id)),
            ("author".to_string(), author),
            ("posted".to_string(), format!("{}", posted)),
            ("content:".to_string(), content),
            ("liked:".to_string(), format!("{}", liked.len())),
            ("pinned:".to_string(), format!("{}", pinned.len())),
            ("bookmarked:".to_string(), format!("{}", bookmarked)),
        ],
    }
}

pub fn build_embed_from_resp(
    ce: &mut CreateEmbed,
    Response {
        title,
        rgb,
        description,
        mut fields,
    }: Response,
) -> &mut CreateEmbed {
    let (r, g, b) = rgb;

    ce.title(title)
        .colour(Colour::from_rgb(r, g, b))
        .description(description)
        .fields(
            fields
                .drain(..)
                .map(|(s1, s2)| (s1, s2, false))
                .collect::<Vec<_>>(),
        )
}

pub fn append_message_reference(
    raw: &mut HashMap<&str, Value>,
    id: MessageId,
    channel_id: ChannelId,
    guild_id: Option<GuildId>,
) {
    let mr = dbg!(json!({
        "message_id": id,
        "channel_id": channel_id,
        "guild_id": match guild_id {
            Some(i) => Value::Number(Number::from(i.0)),
            None => Value::Null
        },
    }));

    dbg!(raw.insert("message_reference", mr));
}

/// parser of "range" notation like of rust's (but, limited).
///
/// 3 notations are available:
///
/// - `[num]..`
///   - `target >= [num]`
///   - [`Over`]
/// - `[num]`
///   - `target == [num]`
///   - [`Eq`]
/// - `..=[num]`
///   - `target <= [num]`
///   - [`Under`]
///
/// and, the following differences are acceptable:
///
/// - spaces before and after.
/// - spaces between `[num]` and *range token* (e.g. `..=`)
///
/// [`Over`]: crate::repositories::Comparison::Over
/// [`Eq`]: crate::repositories::Comparison::Eq
/// [`Under`]: crate::repositories::Comparison::Under
pub fn range_syntax_parser(mut src: String) -> Result<(u32, Comparison)> {
    let mut iter = src.drain(..).enumerate();

    let mut parsing_num = false;
    let mut parsing_range = false;
    let mut before_char = None;

    let mut comp = Comparison::Eq;
    let mut num_raw = String::new();

    loop {
        let (i, c) = match before_char {
            None => match iter.next() {
                Some(t) => t,
                None => break,
            },
            Some(t) => {
                before_char = None;
                t
            },
        };

        dbg!((i, c));

        match c {
            ' ' => (),
            '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                if parsing_num {
                    bail!("no expected char: '{}' (pos: {})", c, i);
                }
                parsing_num = true;

                num_raw.push(c);

                before_char = loop {
                    let (i, c) = match iter.next() {
                        None => break None,
                        Some(t) => t,
                    };

                    match c {
                        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' =>
                            num_raw.push(c),
                        _ => break Some((i, c)),
                    }
                };
            },
            '.' => {
                if parsing_range {
                    bail!("no expected char: '{}' (pos: {})", c, i);
                }
                parsing_range = true;

                let (i, c) = match before_char {
                    None => match iter.next() {
                        Some(t) => t,
                        None => break,
                    },
                    Some(t) => {
                        before_char = None;
                        t
                    },
                };

                match c {
                    '.' => (),
                    _ => bail!("no expected char: '{}' (pos: {})", c, i),
                }

                let (i, c) = match iter.next() {
                    None => {
                        comp = Comparison::Over;
                        break;
                    },
                    Some(t) => t,
                };

                match (c == '=', parsing_num) {
                    (true, false) => comp = Comparison::Under,
                    (false, true) => comp = Comparison::Over,
                    _ => bail!("no expected char: '{}', (pos: {})", c, i),
                }
            },
            _ => bail!("no expected char: '{}' (pos: {})", c, i),
        }
    }

    let num = num_raw.parse()?;

    Ok((num, comp))
}

#[test]
fn parsing_test() {
    use Comparison::*;

    assert_eq!(range_syntax_parser("2..".to_string()).unwrap(), (2, Over));

    assert_eq!(range_syntax_parser("010".to_string()).unwrap(), (10, Eq));

    assert_eq!(range_syntax_parser("..=5".to_string()).unwrap(), (5, Under));

    assert!(range_syntax_parser("..5".to_string()).is_err());

    assert!(range_syntax_parser("3..=".to_string()).is_err());

    assert!(range_syntax_parser("..=5f".to_string()).is_err());

    assert!(range_syntax_parser(".a.2".to_string()).is_err());

    assert!(range_syntax_parser("not expected".to_string()).is_err());

    assert_eq!(
        range_syntax_parser("  ..=   100".to_string()).unwrap(),
        (100, Under)
    );

    assert!(range_syntax_parser(" . . = 100".to_string()).is_err());

    assert_eq!(
        range_syntax_parser("  100    ..".to_string()).unwrap(),
        (100, Over)
    );
}
