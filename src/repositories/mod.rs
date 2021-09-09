use std::collections::HashSet;
use std::ops::Bound;

use async_trait::async_trait;
use regex::Regex;

use crate::entities::{Author, Content, ContentId, Date, User, UserId};

pub mod mock;
pub mod mongo;

type Result<T> = ::std::result::Result<T, RepositoryError>;

#[async_trait]
pub trait UserRepository {
    async fn insert(&self, item: User) -> Result<bool>;
    async fn is_exists(&self, id: UserId) -> Result<bool>;

    async fn find(&self, id: UserId) -> Result<User>;
    async fn finds(&self, query: UserQuery) -> Result<Vec<User>>;

    async fn update(&self, id: UserId, mutation: UserMutation) -> Result<User>;

    async fn is_bookmark(&self, id: UserId, content_id: ContentId) -> Result<bool>;
    async fn insert_bookmark(&self, id: UserId, content_id: ContentId) -> Result<bool>;
    async fn delete_bookmark(&self, id: UserId, content_id: ContentId) -> Result<bool>;

    async fn delete(&self, id: UserId) -> Result<User>;
}

#[async_trait]
pub trait ContentRepository {
    async fn insert(&self, item: Content) -> Result<bool>;
    async fn is_exists(&self, id: ContentId) -> Result<bool>;

    async fn find(&self, id: ContentId) -> Result<Content>;
    async fn finds(&self, query: ContentQuery) -> Result<Vec<Content>>;

    async fn update(&self, id: ContentId, mutation: ContentMutation) -> Result<Content>;

    async fn is_liked(&self, id: ContentId, user_id: UserId) -> Result<bool>;
    async fn insert_liked(&self, id: ContentId, user_id: UserId) -> Result<bool>;
    async fn delete_liked(&self, id: ContentId, user_id: UserId) -> Result<bool>;

    async fn is_pinned(&self, id: ContentId, user_id: UserId) -> Result<bool>;
    async fn insert_pinned(&self, id: ContentId, user_id: UserId) -> Result<bool>;
    async fn delete_pinned(&self, id: ContentId, user_id: UserId) -> Result<bool>;

    async fn delete(&self, id: ContentId) -> Result<Content>;
}

#[derive(Debug, Clone, Default)]
pub struct UserQuery {
    pub bookmark: Option<HashSet<ContentId>>,
    pub bookmark_num: Option<(Bound<u32>, Bound<u32>)>,
}

#[derive(Debug, Clone, Default)]
pub struct ContentQuery {
    pub author: Option<AuthorQuery>,
    pub posted: Option<PostedQuery>,
    pub content: Option<Regex>,
    pub liked: Option<HashSet<UserId>>,
    pub liked_num: Option<(Bound<u32>, Bound<u32>)>,
    pub pinned: Option<HashSet<UserId>>,
    pub pinned_num: Option<(Bound<u32>, Bound<u32>)>,
    // FiF: times query
}

#[derive(Debug, Clone)]
pub enum PostedQuery {
    UserId(UserId),
    UserName(Regex),
    UserNick(Regex),
    Any(Regex),
}

#[derive(Debug, Clone)]
pub enum AuthorQuery {
    UserId(UserId),
    UserName(Regex),
    UserNick(Regex),
    Virtual(Regex),
    Any(Regex),
}

#[derive(Debug)]
pub enum RepositoryError {
    NotFound,
    NoUnique { matched: u32 },
    Internal(anyhow::Error),
}

impl ::std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match self {
            RepositoryError::NotFound => write!(f, "cannot find object."),
            RepositoryError::NoUnique { matched } => write!(
                f,
                "expected unique object, found non-unique objects (matched: {})",
                matched
            ),
            RepositoryError::Internal(e) => write!(f, "internal error: {}", e),
        }
    }
}
impl ::std::error::Error for RepositoryError {}

#[derive(Debug, Clone, Default)]
pub struct UserMutation {
    pub admin: Option<bool>,
    pub sub_admin: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ContentMutation {
    pub author: Option<Author>,
    pub content: Option<ContentContentMutation>,
    pub edited: Date,
}

#[derive(Debug, Clone)]
pub enum ContentContentMutation {
    Complete(String),
    Sed { capture: Regex, replace: String },
}
