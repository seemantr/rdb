quick_error! {
    #[derive(Debug)]
    pub enum DbError {
        DatabaseNotOpen {
            description("Database not open. DatabaseNotOpen is returned when a DB instance is accessed before it is opened or after it is closed.")
            display("Database not open")
        }
        DatabaseOpen {
            description("Database already open. DatabaseOpen is returned when opening a database that is already open.")
            display("database already open")
        }
        Invalid {
            description("Invalid database. Invalid is returned when both meta pages on a database are invalid. This typically occurs when a file is not a rdb database.")
            display("Invalid database")
        }
        VersionMismatch {
            description("Version mismatch. VersionMismatch is returned when the data file was created with a different version of Bolt.")
            display("Version mismatch")
        }
        Checksum {
            description("Checksum error. Checksum is returned when either meta page checksum does not match.")
            display("Checksum error")
        }
    }
}