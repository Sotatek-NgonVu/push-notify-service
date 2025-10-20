use async_trait::async_trait;
use futures::stream::TryStreamExt;
use serde::{de::DeserializeOwned, ser::Serialize};
use std::fmt::Debug;
use validator::Validate;
use wither::Model as WitherModel;
use wither::ModelCursor;
use wither::bson::Bson;
use wither::bson::Document;
use wither::bson::doc;
use wither::bson::from_bson;
use wither::bson::{self, oid::ObjectId};
use wither::mongodb::Database;
use wither::mongodb::options::FindOneAndUpdateOptions;
use wither::mongodb::options::FindOneOptions;
use wither::mongodb::options::FindOptions;
use wither::mongodb::options::ReturnDocument;
use wither::mongodb::options::UpdateOptions;
use wither::mongodb::results::DeleteResult;
use wither::mongodb::results::UpdateResult;

use crate::errors::Error;

use super::pagination::PaginationQuery;
use super::pagination::PaginationResponseDto;

// This is the Model trait. All models that have a MongoDB collection should
// implement this and therefore inherit theses methods.
#[allow(dead_code)]
#[async_trait]
pub trait ModelExt
where
    Self: WitherModel + Validate,
{
    async fn get_connection() -> &'static Database;

    async fn create(mut model: Self) -> Result<Self, Error> {
        let connection = Self::get_connection().await;
        model
            .validate()
            .map_err(|e| Error::bad_request(&format!("Validation db error: {e:?}")))?;
        model.save(connection, None).await.map_err(Error::Wither)?;

        Ok(model)
    }

    async fn find_by_id(id: &ObjectId) -> Result<Option<Self>, Error> {
        let connection = Self::get_connection().await;
        <Self as WitherModel>::find_one(connection, doc! { "_id": id }, None)
            .await
            .map_err(Error::Wither)
    }

    async fn find_one<O>(query: Document, options: O) -> Result<Option<Self>, Error>
    where
        O: Into<Option<FindOneOptions>> + Send,
    {
        let connection = Self::get_connection().await;
        <Self as WitherModel>::find_one(connection, query, options)
            .await
            .map_err(Error::Wither)
    }

    async fn find<O>(query: Document, options: O) -> Result<Vec<Self>, Error>
    where
        O: Into<Option<FindOptions>> + Send,
    {
        let connection = Self::get_connection().await;
        <Self as WitherModel>::find(connection, query, options)
            .await
            .map_err(Error::Wither)?
            .try_collect::<Vec<Self>>()
            .await
            .map_err(Error::Wither)
    }

    async fn find_and_count<O>(query: Document, options: O) -> Result<(Vec<Self>, u64), Error>
    where
        O: Into<Option<FindOptions>> + Send,
    {
        let connection = Self::get_connection().await;

        let count = Self::collection(connection)
            .count_documents(query.clone())
            .await
            .map_err(Error::Mongo)?;

        let items = <Self as WitherModel>::find(connection, query, options.into())
            .await
            .map_err(Error::Wither)?
            .try_collect::<Vec<Self>>()
            .await
            .map_err(Error::Wither)?;

        Ok((items, count))
    }

    async fn cursor<O>(query: Document, options: O) -> Result<ModelCursor<Self>, Error>
    where
        O: Into<Option<FindOptions>> + Send,
    {
        let connection = Self::get_connection().await;
        <Self as WitherModel>::find(connection, query, options)
            .await
            .map_err(Error::Wither)
    }

    async fn find_one_and_update(
        query: Document,
        update: Document,
        upsert: bool,
    ) -> Result<Option<Self>, Error> {
        let connection = Self::get_connection().await;
        let options = FindOneAndUpdateOptions::builder()
            .upsert(upsert)
            .return_document(ReturnDocument::After)
            .build();

        <Self as WitherModel>::find_one_and_update(connection, query, update, options)
            .await
            .map_err(Error::Wither)
    }

    async fn update_one<O>(
        query: Document,
        update: Document,
        options: O,
    ) -> Result<UpdateResult, Error>
    where
        O: Into<Option<UpdateOptions>> + Send,
    {
        let connection = Self::get_connection().await;
        Self::collection(connection)
            .update_one(query, update)
            .with_options(options)
            .await
            .map_err(Error::Mongo)
    }

    async fn update_many<O>(
        query: Document,
        update: Document,
        options: O,
    ) -> Result<UpdateResult, Error>
    where
        O: Into<Option<UpdateOptions>> + Send,
    {
        let connection = Self::get_connection().await;
        Self::collection(connection)
            .update_many(query, update)
            .with_options(options)
            .await
            .map_err(Error::Mongo)
    }

    async fn delete_many(query: Document) -> Result<DeleteResult, Error> {
        let connection = Self::get_connection().await;
        <Self as WitherModel>::delete_many(connection, query, None)
            .await
            .map_err(Error::Wither)
    }

    async fn delete_one(query: Document) -> Result<DeleteResult, Error> {
        let connection = Self::get_connection().await;
        Self::collection(connection)
            .delete_one(query)
            .await
            .map_err(Error::Mongo)
    }

    async fn count(query: Document) -> Result<u64, Error> {
        let connection = Self::get_connection().await;
        Self::collection(connection)
            .count_documents(query)
            .await
            .map_err(Error::Mongo)
    }

    async fn exists(query: Document) -> Result<bool, Error> {
        let connection = Self::get_connection().await;
        let count = Self::collection(connection)
            .count_documents(query)
            .await
            .map_err(Error::Mongo)?;

        Ok(count > 0)
    }

    async fn aggregate<A>(pipeline: Vec<Document>) -> Result<Vec<A>, Error>
    where
        A: Serialize + DeserializeOwned + std::fmt::Debug,
    {
        let connection = Self::get_connection().await;

        let documents = Self::collection(connection)
            .aggregate(pipeline.clone())
            .await
            .map_err(Error::Mongo)?
            .try_collect::<Vec<Document>>()
            .await
            .map_err(Error::Mongo)?;

        let documents = documents
            .into_iter()
            .enumerate()
            .map(|(index, document)| {
                from_bson::<A>(Bson::Document(document.clone())).map_err(|e| {
                    tracing::error!(
            "Failed to deserialize document at index {}: error={:?}, document={:?}, type={}",
            index,
            e,
            document,
            std::any::type_name::<A>()
          );
                    e
                })
            })
            .collect::<Result<Vec<A>, bson::de::Error>>()
            .map_err(|e| {
                tracing::error!(
                    "Aggregate deserialization failed: error={:?}, target_type={}, pipeline={:?}",
                    e,
                    std::any::type_name::<A>(),
                    pipeline
                );
                Error::SerializeMongoResponse(e)
            })?;
        Ok(documents)
    }

    // TODO: Better performance ?
    async fn paginate<A>(
        query: Document,
        pipeline: Vec<Document>,
        pagination: &PaginationQuery,
    ) -> Result<PaginationResponseDto<A>, Error>
    where
        A: Serialize + DeserializeOwned + Send + Debug,
    {
        let start_time = std::time::Instant::now();
        let page = pagination.page();
        let limit = pagination.limit();
        let skip = pagination.skip();

        let mut full_pipeline = vec![doc! { "$match": query.clone() }];
        full_pipeline.extend(pipeline);

        let mut count_pipeline = full_pipeline.clone();
        count_pipeline.push(doc! { "$count": "total" });
        // let count_result: Vec<Document> = Self::aggregate(count_pipeline.clone()).await?;
        let count_pipeline_time = std::time::Instant::now();
        let duration = count_pipeline_time.duration_since(start_time);
        if duration.as_millis() > 1000 {
            tracing::warn!(
                "[warning_processing_time] paginate count pipeline time taken: {:?}, pipeline={:?}",
                duration,
                count_pipeline
            );
        }

        // TODO: currently disable pagination due to the performance issue
        // We'll come up with a better pagination with cursor-style in the future
        // let total_docs = count_result
        //   .first()
        //   .and_then(|doc| doc.get_i32("total").ok())
        //   .map(|count| count as u32)
        //   .unwrap_or(0);
        // let total_pages = total_docs.div_ceil(limit);
        let total_docs: u32 = 0;
        let total_pages: u32 = 0;
        full_pipeline.push(doc! { "$skip": skip });
        full_pipeline.push(doc! { "$limit": limit });

        let result: Vec<A> = Self::aggregate(full_pipeline.clone()).await?;

        let full_pipeline_time = std::time::Instant::now();
        let duration = full_pipeline_time.duration_since(count_pipeline_time);
        if duration.as_millis() > 1000 {
            tracing::warn!(
                "[warning_processing_time] paginate full pipeline time taken: {:?}, time from start={:?}, pipeline={:?}",
                duration,
                full_pipeline_time.duration_since(start_time),
                full_pipeline
            );
        }

        Ok(PaginationResponseDto {
            total_docs,
            total_pages,
            page,
            limit,
            docs: result,
        })
    }
}
