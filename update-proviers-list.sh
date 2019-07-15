wget http://providers.delta.chat/data.xml
echo 'static PROVIDERS_XML: &str = r#"' > src/provider-xml.rs
cat data.xml >> src/provider-xml.rs
echo '"#;' >> src/provider-xml.rs
rm data.xml
