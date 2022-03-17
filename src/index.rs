use serde::{Serialize, Deserialize};
use tantivy::{
    schema::{
        Schema as TantivySchema,
        Document as TantivyDocument,
        STORED,
        TEXT,
    }, 
    DocAddress,
    Index as TantivyIndex,
    collector::TopDocs,
    query::QueryParser,
    ReloadPolicy,
};

use super::hashmap_directory::HashMapDirectory;
use wasm_bindgen::prelude::*;


#[wasm_bindgen]
#[derive(Serialize, Deserialize, Clone)]
pub struct Schema{
    fields: Vec<String>,
    stored_fields: Vec<String>,
}

#[wasm_bindgen]
impl Schema {

    pub fn new()-> Schema{
        Schema{
            fields: Vec::new(),
            stored_fields: Vec::new(),
        }
    }

    pub fn add_field(&mut self, field: &str){
        self.fields.push(field.to_string());
    }
    
    pub fn add_stored_field(&mut self, field: &str){
        self.stored_fields.push(field.to_string());
    }

    fn build_schema(&self)-> TantivySchema{
        let mut schema_builder = TantivySchema::builder();
        for field in self.fields.iter(){
            schema_builder.add_text_field(&field, TEXT);
        }
        for stored_field in self.stored_fields.iter() {
            schema_builder.add_text_field(&stored_field, TEXT | STORED);
        }
        let schema = schema_builder.build();
        schema
    }

    fn get_field_names(&self)-> Vec<String>{
        let mut fields = Vec::new();
        for field in self.fields.iter(){
            fields.push(field.clone());
        }
        for stored_field in self.stored_fields.iter() {
            fields.push(stored_field.clone());
        }
        fields
    }
}

#[derive(Serialize, Deserialize)]
struct SerializableIndex {
    schema: Schema,
    directory: HashMapDirectory,
}

#[wasm_bindgen]
pub struct Index{
    tantivy_index: TantivyIndex,
    tantivy_schema: TantivySchema,
    schema: Schema,
    directory: HashMapDirectory,
}

#[wasm_bindgen]
impl Index {
    pub fn parse_serialized_index(serialized_index: &str)-> Index{
        let SerializableIndex{schema, directory} = serde_json::from_str(serialized_index).unwrap();
        let tantivy_schema = schema.build_schema();
        let tantivy_index = TantivyIndex::builder().schema(tantivy_schema.clone()).open_or_create(directory.clone()).unwrap();
        Index { tantivy_index, tantivy_schema, schema, directory }
    }
    
    pub fn from_schema(schema: Schema) -> Index{
        let directory= HashMapDirectory::new();
        let tantivy_schema = schema.build_schema();
        let tantivy_index = TantivyIndex::builder().schema(tantivy_schema.clone()).open_or_create(directory.clone()).unwrap();

        Index { tantivy_index, tantivy_schema, schema, directory }
    }

    pub fn serialize_index(&self) -> String {
        let serializable_index = SerializableIndex {
            schema: self.schema.clone(),
            directory: self.directory.clone(),
        };
        serde_json::to_string(&serializable_index).unwrap()
    }

    pub fn add_document(&self, doc: Document){
        let tantivy_doc = doc.get_tantivy_document(&self.tantivy_schema);
        let mut index_writer = self.tantivy_index.writer(50_000_000).unwrap(); // TODO est-ce que c'est pertinent de le re-créer à chaque fois
        index_writer.add_document(tantivy_doc);
        index_writer.commit().unwrap();// TODO est-ce que c'est pertinent de le re-commiter à chaque fois
    }

    pub fn search(&self, query: &str) -> String {
        let reader = self.tantivy_index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into().unwrap();

        let searcher = reader.searcher();
        let fields = self.schema.get_field_names().iter().map(|field_name| self.tantivy_schema.get_field(&field_name).unwrap()).collect();
        let query_parser = QueryParser::for_index(&self.tantivy_index, fields);
        let query = query_parser.parse_query(query).unwrap();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();

        let mut results_string = String::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address).unwrap();
            results_string.push_str(&self.tantivy_schema.to_json(&retrieved_doc));
        }
        results_string
    }
}

#[wasm_bindgen]
pub struct Document{
        texts:Vec<(String, String)>,
}

#[wasm_bindgen]
impl Document{
    fn get_tantivy_document(self, tantivy_schema: &TantivySchema) -> TantivyDocument{
        let mut doc = TantivyDocument::default();
        for (field_name, data) in self.texts {
            let field = tantivy_schema.get_field(&field_name).unwrap(); //TODO deal with errors here
            doc.add_text(field, data);
        }
        doc
    }

    pub fn new()-> Self{
        Document { texts: Vec::new() }
    }

    pub fn add_text(&mut self, field: &str, data: &str){
        self.texts.push((field.to_string(), data.to_string()));
    }
}