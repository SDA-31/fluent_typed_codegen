use crate::schema;

#[test]
fn schema_rejects_changed_variables_and_duplicate_messages() {
	let expected = "hello = Hello { $name }\n";
	assert!(schema::validate("hello = Hallo { $name }\n", expected).is_ok());
	assert!(schema::validate("hello = Hallo { $other }\n", expected).is_err());
	assert!(schema::validate("hello = A\nhello = B\n", expected).is_err());
}
