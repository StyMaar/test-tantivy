
use std::collections::HashMap;

use tantivy::{
    schema::{
        Schema as TantivySchema,
        Document as TantivyDocument,
        FAST,
        INDEXED,
        STRING,
        STORED,
        TEXT,
        TextOptions,
    },
    DocAddress,
    Index as TantivyIndex,
    collector::TopDocs,
    query::QueryParser,
    ReloadPolicy,
};

use rkyv::{Archive, Deserialize, Serialize};
use serde::{Serialize as SerdeSerialize, Deserialize as SerializeDeserialize};

use bytecheck::CheckBytes;

use super::hashmap_directory::{HashMapDirectory, SerializableHashMapDirectory};
use wasm_bindgen::prelude::*;


#[wasm_bindgen]
#[derive(Archive, Serialize, Deserialize, Clone, SerdeSerialize, SerializeDeserialize)]
// To use the safe API, you have to derive CheckBytes for the archived type
#[archive_attr(derive(CheckBytes, Debug))]
pub struct FieldOption {
    fast: Option<bool>,
    indexed: Option<bool>,
    string: Option<bool>,
    stored: Option<bool>,
    text: Option<bool>,
} 

#[wasm_bindgen]
#[derive(Archive, Serialize, Deserialize, Clone, SerdeSerialize, SerializeDeserialize)]
// To use the safe API, you have to derive CheckBytes for the archived type
#[archive_attr(derive(CheckBytes, Debug))]
pub struct Schema{
    fields: HashMap<String, FieldOption>,
}

#[wasm_bindgen]
impl Schema {

    #[wasm_bindgen(constructor)]
    pub fn new()-> Schema{
        Schema{
            fields: HashMap::new(),
        }
    }
    
    #[wasm_bindgen(js_name = "addField")]
    pub fn add_field(&mut self, field_name: &str, js_field: JsValue){
        let field = serde_wasm_bindgen::from_value(js_field).unwrap(); //TODO handle error here
        self.fields.insert(field_name.to_string(), field);
    }


    fn build_schema(&self)-> TantivySchema{
        let mut schema_builder = TantivySchema::builder();
        for (field_name, option) in self.fields.iter(){
            
            // TODO implement the field options in a way that makes sense
            let field_option = TextOptions::default();
            // if Some(true) = option.fast {
            //     field_option = field_option | FAST;
            // }
            // if Some(true) = option.indexed {
            //     field_option = field_option | INDEXED;
            // }
            // if Some(true) = option.string {
            //     field_option = field_option | STRING;
            // }
            // if Some(true) = option.stored {
            //     field_option = field_option | STORED;
            // }
            // if Some(true) = option.text {
            //     field_option = field_option | TEXT;
            // }

            schema_builder.add_text_field(field_name, TEXT | STORED);
        }
        let schema = schema_builder.build();
        schema
    }

    #[wasm_bindgen(js_name = "getFields")]
    pub fn get_fields(&self)-> JsValue{
        serde_wasm_bindgen::to_value(self).unwrap()
    }
}

#[derive(Archive, Serialize, Deserialize)]
// To use the safe API, you have to derive CheckBytes for the archived type
#[archive_attr(derive(CheckBytes, Debug))]
struct SerializableIndex {
    schema: Schema,
    directory: SerializableHashMapDirectory,
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

    #[wasm_bindgen(js_name = "parseSerializedIndex")]
    pub fn parse_serialized_index(serialized_index: &[u8])-> Index{
        let archived = rkyv::check_archived_root::<SerializableIndex>(serialized_index).unwrap();
        let SerializableIndex{schema, directory} = archived.deserialize(&mut rkyv::Infallible).unwrap();
        let directory: HashMapDirectory = directory.into();

        let tantivy_schema = schema.build_schema();
        let tantivy_index = TantivyIndex::builder().schema(tantivy_schema.clone()).open_or_create(directory.clone()).unwrap();
        Index { tantivy_index, tantivy_schema, schema, directory}
    }

    #[wasm_bindgen(js_name = "fromSchema")]
    pub fn from_schema(schema: Schema) -> Index{
        let directory= HashMapDirectory::new();
        let tantivy_schema = schema.build_schema();
        let tantivy_index = TantivyIndex::builder().schema(tantivy_schema.clone()).open_or_create(directory.clone()).unwrap();

        Index { tantivy_index, tantivy_schema, schema, directory }
    }

    #[wasm_bindgen(js_name = "serializeIndex")]
    pub fn serialize_index(&self) -> Vec<u8> {
        let serializable_index = SerializableIndex {
            schema: self.schema.clone(),
            directory: (&self.directory).into(),
        };
        let bytes = rkyv::to_bytes::<_, 256>(&serializable_index).unwrap().into_vec();
        bytes
    }

    #[wasm_bindgen(js_name = "addDocument")]
    pub fn add_document(&self, doc: Document){
        let tantivy_doc = doc.get_tantivy_document(&self.tantivy_schema);
        let mut index_writer = self.tantivy_index.writer(50_000_000).unwrap(); // TODO est-ce que c'est pertinent de le re-créer à chaque fois
        index_writer.add_document(tantivy_doc);
        index_writer.commit().unwrap();// TODO est-ce que c'est pertinent de le re-commiter à chaque fois
    }

    #[wasm_bindgen(js_name = "search")]
    pub fn search(&self, query: &str) -> String {
        let reader = self.tantivy_index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into().unwrap();

        let searcher = reader.searcher();
        let fields = self.schema.fields.keys().map(|field_name| self.tantivy_schema.get_field(&field_name).unwrap()).collect();
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

    #[wasm_bindgen(constructor)]
    pub fn new()-> Self{
        Document { texts: Vec::new() }
    }

    #[wasm_bindgen(js_name = "addText")]
    pub fn add_text(&mut self, field: &str, data: &str){
        self.texts.push((field.to_string(), data.to_string()));
    }
}