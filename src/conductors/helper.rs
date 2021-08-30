use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::io::Cursor;
use std::num::ParseIntError;
use std::ops::Bound;
use std::str::FromStr;

use anyhow::{anyhow, bail, Error, Result};
use clap::ErrorKind;
use serde_json::{json, Number, Value};
use serenity::builder::CreateEmbed;
use serenity::model::id::{ChannelId, GuildId, MessageId, UserId};
use serenity::model::interactions::application_command::ApplicationCommandInteractionData;
use serenity::utils::Colour;
use uuid::Uuid;

use super::{clapcmd, command_strs, Command, CommandV2, MsgCommand, Response};
use crate::conductors::UserCommandV2;
use crate::entities::{Content, User};
use crate::repositories::{Comparison, ContentQuery};

#[deprecated]
pub async fn parse_ia(_: &ApplicationCommandInteractionData) -> Result<Command> {
    unimplemented!();
}

pub async fn parse_msg_v2(msg: &str) -> Option<Result<CommandV2, String>> {
    let res: Result<_> = try {
        let splitted = shell_words::split(msg)?;

        if let Some(n) = splitted.get(0) {
            if n != "*ip" {
                return None;
            }
        }

        let ams0 = match clapcmd::create_clap_app_v2().get_matches_from_safe(splitted) {
            Ok(o) => o,
            Err(e) => match e.kind {
                ErrorKind::VersionDisplayed => Err(anyhow!({
                    let mut buf = Cursor::new(vec![]);
                    clapcmd::create_clap_app_v2()
                        .write_long_version(&mut buf)
                        .unwrap();
                    String::from_utf8(buf.into_inner()).unwrap()
                }))?,
                _ => Err(anyhow!(e))?,
            },
        };

        match ams0.subcommand() {
            ("user", Some(ams1)) => CommandV2::User(match ams1.subcommand() {
                ("create", Some(_)) => UserCommandV2::Create,
                ("read", Some(ams2)) => {
                    let mut errs = vec![];

                    let id = ams2.value_of("id").map(|s| match s.parse() {
                        Ok(o) => o,
                        Err(e) => {
                            let e: ParseIntError = e;
                            errs.push(e.to_string());
                            0 // tmp value
                        },
                    });

                    if !errs.is_empty() {
                        Err(anyhow!(combine_errs(errs)))?
                    }

                    UserCommandV2::Read { id }
                },
                ("reads", Some(ams2)) => {
                    unimplemented!()
                },
                ("update", Some(ams2)) => {
                    unimplemented!()
                },
                sc => unreachable!("unrecognized subcommand on `user`. (impl error): {:?}", sc),
            }),
            ("content", Some(ams1)) => CommandV2::Content(match ams1.subcommand() {
                ("read", Some(ams2)) => {
                    unimplemented!()
                },
                ("reads", Some(ams2)) => {
                    unimplemented!()
                },
                ("update", Some(ams2)) => {
                    unimplemented!()
                },
                ("delete", Some(ams2)) => {
                    unimplemented!()
                },
                sc => unreachable!(
                    "unrecognized subcommand on `content`. (impl error): {:?}",
                    sc
                ),
            }),
            ("post", Some(ams1)) => {
                unimplemented!()
            },
            ("like", Some(ams1)) => {
                unimplemented!()
            },
            ("pin", Some(ams1)) => {
                unimplemented!()
            },
            ("bookmark", Some(ams1)) => {
                unimplemented!()
            },
            sc => unreachable!(
                "unrecognized subcommand on `(root)`. (impl error): {:?}",
                sc
            ),
        }
    };

    let tmp = match res {
        Ok(o) => Ok(o),
        Err(e) => Err(e.to_string()),
    };

    Some(tmp)
}

pub async fn parse_msg(msg: &str) -> Option<MsgCommand> {
    let res: Result<_> = try {
        let splitted = shell_words::split(msg)?;

        if let Some(n) = splitted.get(0) {
            if n != command_strs::PREFIX {
                return None;
            }
        }

        let ams = match clapcmd::create_clap_app().get_matches_from_safe(splitted) {
            Ok(o) => o,
            Err(e) => match e.kind {
                ErrorKind::VersionDisplayed =>
                    return Some(MsgCommand::Showing({
                        let mut buf = Cursor::new(vec![]);
                        clapcmd::create_clap_app()
                            .write_long_version(&mut buf)
                            .unwrap();
                        String::from_utf8(buf.into_inner()).unwrap()
                    })),
                _ => return Some(MsgCommand::Showing(e.message)),
            },
        };

        use command_strs::*;

        use super::clapcmd::{extract_clap_arg, extract_clap_sams};

        let cmd = match match ams.subcommand_name() {
            None => return None,
            Some(s) => s,
        } {
            register::NAME => Command::UserRegister,
            info::NAME => Command::UserRead,
            change::NAME => {
                let sams = extract_clap_sams(&ams, change::NAME).unwrap();
                let admin_raw = sams.value_of(change::admin::NAME);
                let sub_admin_raw = sams.value_of(change::sub_admin::NAME);

                let admin = match admin_raw {
                    Some(s) => Some(bool::from_str(s)?),
                    None => None,
                };

                let sub_admin = match sub_admin_raw {
                    Some(s) => Some(bool::from_str(s)?),
                    None => None,
                };

                Command::UserUpdate { admin, sub_admin }
            },
            bookmark::NAME => {
                let sams = extract_clap_sams(&ams, bookmark::NAME).unwrap();
                let content_id =
                    Uuid::from_str(extract_clap_arg(sams, bookmark::id::NAME).unwrap())?;
                let undo = sams.values_of(bookmark::undo::NAME).is_some();

                Command::Bookmark { content_id, undo }
            },
            delete_me::NAME => Command::UserDelete,
            post::NAME => {
                let sams = extract_clap_sams(&ams, post::NAME).unwrap();
                let content = extract_clap_arg(sams, post::content::NAME)
                    .unwrap()
                    .to_string();
                let author = extract_clap_arg(sams, post::author::NAME)
                    .unwrap()
                    .to_string();

                Command::ContentPost { content, author }
            },
            get::NAME => {
                let sams = extract_clap_sams(&ams, get::NAME).unwrap();

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
                let page = match extract_clap_arg(sams, get::page::NAME) {
                    Ok(o) => o.parse()?,
                    Err(e) => {
                        dbg!(e);
                        1
                    },
                };

                Command::ContentRead { queries, page }
            },
            edit::NAME => {
                let sams = extract_clap_sams(&ams, edit::NAME).unwrap();
                let content_id = Uuid::from_str(extract_clap_arg(sams, edit::id::NAME).unwrap())?;
                let new_content = extract_clap_arg(sams, edit::content::NAME)
                    .unwrap()
                    .to_string();

                Command::ContentUpdate {
                    content_id,
                    new_content,
                }
            },
            like::NAME => {
                let sams = extract_clap_sams(&ams, like::NAME).unwrap();
                let content_id = Uuid::from_str(extract_clap_arg(sams, like::id::NAME).unwrap())?;
                let undo = sams.values_of(like::undo::NAME).is_some();

                Command::Like { content_id, undo }
            },
            pin::NAME => {
                let sams = extract_clap_sams(&ams, pin::NAME).unwrap();
                let content_id = Uuid::from_str(extract_clap_arg(sams, pin::id::NAME).unwrap())?;
                let undo = sams.values_of(pin::undo::NAME).is_some();

                Command::Pin { content_id, undo }
            },
            remove::NAME => {
                let sams = extract_clap_sams(&ams, remove::NAME).unwrap();
                let content_id = Uuid::from_str(extract_clap_arg(sams, remove::id::NAME).unwrap())?;

                Command::ContentDelete { content_id }
            },
            _ => panic!("unrecognized subcommand. (impl error)"),
        };

        Some(MsgCommand::Command(cmd))
    };

    match res {
        Ok(o) => o,
        Err(e) => Some(MsgCommand::Showing(e.to_string())),
    }
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
                .map(|(s1, s2)| (s1, s2, true))
                .collect::<Vec<_>>(),
        )
}

pub fn append_message_reference(
    raw: &mut HashMap<&str, Value>,
    id: MessageId,
    channel_id: ChannelId,
    guild_id: Option<GuildId>,
) {
    let mr = json!({
        "message_id": id,
        "channel_id": channel_id,
        "guild_id": match guild_id {
            Some(GuildId(i)) => Value::Number(Number::from(i)),
            None => Value::Null
        },
    });

    raw.insert("message_reference", mr);
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
#[deprecated]
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

pub fn range_syntax_parser_v2<N>(src: String) -> Result<(Bound<N>, Bound<N>)>
where
    N: range_parser::Num + FromStr + Debug,
    <N as FromStr>::Err: Debug + PartialEq + Eq,
{
    range_parser::parse(src).map_err(|e| anyhow::anyhow!("{:?}", e))
}

#[inline]
fn combine_errs(mut errs: Vec<String>) -> String {
    let mut s = vec![];
    let len = errs.len();
    errs.drain(..)
        .enumerate()
        .map(|(i, s)| match (i + 1) == len {
            true => format!("err ({}): {}", i, s),
            false => format!("err ({}): {}\n", i, s),
        })
        .map(|v| v.into_bytes())
        .for_each(|mut v| s.append(&mut v));

    String::from_utf8(s).unwrap()
}
