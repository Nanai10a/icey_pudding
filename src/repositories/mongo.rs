use anyhow::anyhow;
use async_trait::async_trait;
use mongodb::bson::doc;
use mongodb::{Collection, Database};
use serde::{Deserialize, Serialize};
use serenity::futures::StreamExt;
use uuid::Uuid;

use super::{ContentRepository, RepositoryError, Result, StdResult, UserRepository};
use crate::entities::{Content, User};

pub struct MongoUserRepository {
    main_coll: Collection<MongoUserModel>,
    db: Database,
}

impl MongoUserRepository {
    #[inline]
    fn posted_coll(&self, id: u64) -> Collection<Uuid> {
        self.db.collection(format!("user:{:x}#posted", id).as_str())
    }

    #[inline]
    fn bookmarked_coll(&self, id: u64) -> Collection<Uuid> {
        self.db
            .collection(format!("user:{:x}#bookmarked", id).as_str())
    }
}

pub struct MongoContentRepository {
    main_coll: Collection<MongoContentModel>,
    db: Database,
}

impl MongoContentRepository {
    #[inline]
    fn liked_coll(&self, id: Uuid) -> Collection<u64> {
        self.db
            .collection(format!("content:{:x}#liked", id.as_u128()).as_str())
    }

    #[inline]
    fn pinned_coll(&self, id: Uuid) -> Collection<u64> {
        self.db
            .collection(format!("content:{:x}#pinned", id.as_u128()).as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MongoUserModel {
    id: String,
    admin: bool,
    sub_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MongoContentModel {
    id: Uuid,
    author: MongoContentAuthorModel,
    posted: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum MongoContentAuthorModel {
    User {
        id: String,
        name: String,
        nick: Option<String>,
    },
    Virtual(String),
}

#[async_trait]
impl UserRepository for MongoUserRepository {
    async fn insert(
        &self,
        User {
            id,
            admin,
            sub_admin,
            posted,
            bookmark,
        }: User,
    ) -> Result<bool> {
        let model = MongoUserModel {
            id: id.to_string(),
            admin,
            sub_admin,
        };

        self.main_coll.insert_one(model, None).await.cvt()?;
        match posted.len() {
            0 => (),
            _ => self
                .posted_coll(id)
                .insert_many(posted, None)
                .await
                .cvt()?
                .dispose(),
        }
        match bookmark.len() {
            0 => (),
            _ => self
                .bookmarked_coll(id)
                .insert_many(bookmark, None)
                .await
                .cvt()?
                .dispose(),
        }

        Ok(true)
    }

    async fn is_exists(&self, id: u64) -> Result<bool> {
        match self
            .main_coll
            .count_documents(doc! { "id": id.to_string() }, None)
            .await
            .cvt()?
        {
            0 => Ok(false),
            1 => Ok(true),
            i => Err(RepositoryError::NoUnique { matched: i as u32 }),
        }
    }

    async fn find(&self, id: u64) -> Result<User> { unimplemented!() }

    async fn finds(&self, query: super::UserQuery) -> Result<Vec<User>> { unimplemented!() }

    async fn update(&self, id: u64, mutation: super::UserMutation) -> Result<User> {
        unimplemented!()
    }

    async fn is_posted(&self, id: u64, content_id: Uuid) -> Result<bool> { unimplemented!() }

    async fn insert_posted(&self, id: u64, content_id: Uuid) -> Result<bool> { unimplemented!() }

    async fn delete_posted(&self, id: u64, content_id: Uuid) -> Result<bool> { unimplemented!() }

    async fn is_bookmarked(&self, id: u64, content_id: Uuid) -> Result<bool> { unimplemented!() }

    async fn insert_bookmarked(&self, id: u64, content_id: Uuid) -> Result<bool> {
        unimplemented!()
    }

    async fn delete_bookmarked(&self, id: u64, content_id: Uuid) -> Result<bool> {
        unimplemented!()
    }

    async fn delete(&self, id: u64) -> Result<User> { unimplemented!() }
}

#[async_trait]
impl ContentRepository for MongoContentRepository {
    async fn insert(&self, item: Content) -> Result<bool> { unimplemented!() }

    async fn is_exists(&self, id: Uuid) -> Result<bool> { unimplemented!() }

    async fn find(&self, id: Uuid) -> Result<Content> { unimplemented!() }

    async fn finds(&self, query: super::ContentQuery) -> Result<Vec<Content>> { unimplemented!() }

    async fn update(&self, id: Uuid, mutation: super::ContentMutation) -> Result<Content> {
        unimplemented!()
    }

    async fn is_liked(&self, id: Uuid, user_id: u64) -> Result<bool> { unimplemented!() }

    async fn insert_liked(&self, id: Uuid, user_id: u64) -> Result<bool> { unimplemented!() }

    async fn delete_liked(&self, id: Uuid, user_id: u64) -> Result<bool> { unimplemented!() }

    async fn is_pinned(&self, id: Uuid, user_id: u64) -> Result<bool> { unimplemented!() }

    async fn insert_pinned(&self, id: Uuid, user_id: u64) -> Result<bool> { unimplemented!() }

    async fn delete_pinned(&self, id: Uuid, user_id: u64) -> Result<bool> { unimplemented!() }

    async fn delete(&self, id: Uuid) -> Result<Content> { unimplemented!() }
}

trait Convert<T> {
    fn cvt(self) -> T;
}
impl<T, E: Sync + Send + ::std::error::Error + 'static> Convert<Result<T>> for StdResult<T, E> {
    fn cvt(self) -> Result<T> { self.map_err(|e| RepositoryError::Internal(anyhow!(e))) }
}

trait Dispose {
    fn dispose(self);
}
impl<T> Dispose for T {
    fn dispose(self) {}
}
