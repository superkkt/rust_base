mod mysql;

pub struct Configuration {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub name: String,
}
