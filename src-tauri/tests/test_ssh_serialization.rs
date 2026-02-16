// Test SSH config serialization to frontend
// Run with: cargo test --test test_ssh_serialization

#[cfg(test)]
mod tests {
    use local_ts_lib::database::config::{ConnectionConfig, PostgreSQLConfig};
    use local_ts_lib::database::ssh::{SshConfig, SshAuth};
    use std::path::PathBuf;

    #[test]
    fn test_ssh_config_password_serialization_excludes_password() {
        let config = SshConfig {
            host: "ssh.example.com".to_string(),
            port: 22,
            username: "sshuser".to_string(),
            auth: SshAuth::Password {
                password: "secret_password".to_string(),
            },
            local_port: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        println!("Serialized SSH config (password): {}", json);

        // Verify password is not in the output
        assert!(!json.contains("secret_password"));
        assert!(!json.contains("password"));

        // Verify structure is present
        assert!(json.contains("\"type\":\"Password\""));
        assert!(json.contains("ssh.example.com"));
        assert!(json.contains("sshuser"));
    }

    #[test]
    fn test_ssh_config_key_serialization_excludes_passphrase() {
        let config = SshConfig {
            host: "ssh.example.com".to_string(),
            port: 2222,
            username: "sshuser".to_string(),
            auth: SshAuth::PrivateKey {
                key_path: PathBuf::from("/home/user/.ssh/id_rsa"),
                passphrase: Some("secret_passphrase".to_string()),
            },
            local_port: Some(5432),
        };

        let json = serde_json::to_string(&config).unwrap();
        println!("Serialized SSH config (key): {}", json);

        // Verify passphrase is not in the output
        assert!(!json.contains("secret_passphrase"));
        assert!(!json.contains("passphrase"));

        // Verify structure is present
        assert!(json.contains("\"type\":\"PrivateKey\""));
        assert!(json.contains("key_path"));
        assert!(json.contains("/home/user/.ssh/id_rsa"));
        assert!(json.contains("ssh.example.com"));
    }

    #[test]
    fn test_postgresql_config_with_ssh_serialization() {
        let pg_config = PostgreSQLConfig {
            host: "db.example.com".to_string(),
            port: 5432,
            database: "mydb".to_string(),
            username: "dbuser".to_string(),
            password: Some("db_password".to_string()),
            use_ssl: true,
            ssh_tunnel: Some(SshConfig {
                host: "ssh.example.com".to_string(),
                port: 22,
                username: "sshuser".to_string(),
                auth: SshAuth::Password {
                    password: "ssh_password".to_string(),
                },
                local_port: None,
            }),
        };

        let json = serde_json::to_string(&pg_config).unwrap();
        println!("Serialized PostgreSQL config: {}", json);

        // Verify DB password is not in the output (it's skipped at the field level)
        assert!(!json.contains("db_password"));

        // Verify SSH password is not in the output
        assert!(!json.contains("ssh_password"));

        // Verify SSH structure is present
        assert!(json.contains("ssh_tunnel"));
        assert!(json.contains("ssh.example.com"));
    }

    #[test]
    fn test_workspace_connection_config_roundtrip() {
        // Test that we can serialize and deserialize ConnectionConfig
        let original = ConnectionConfig::PostgreSQL(PostgreSQLConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            username: "testuser".to_string(),
            password: None, // Password should come from keyring
            use_ssl: false,
            ssh_tunnel: Some(SshConfig {
                host: "localhost".to_string(),
                port: 2222,
                username: "sshuser".to_string(),
                auth: SshAuth::Password {
                    password: "sshpass".to_string(),
                },
                local_port: None,
            }),
        });

        // Serialize (this is what happens when sending to frontend)
        let json = serde_json::to_string(&original).unwrap();

        // Verify sensitive data is excluded
        assert!(!json.contains("sshpass"));

        // Deserialize (this is what happens when receiving from frontend)
        let deserialized: ConnectionConfig = serde_json::from_str(&json).unwrap();

        // Verify the structure is preserved
        match deserialized {
            ConnectionConfig::PostgreSQL(pg) => {
                assert_eq!(pg.host, "localhost");
                assert_eq!(pg.database, "testdb");
                assert!(pg.ssh_tunnel.is_some());

                let ssh = pg.ssh_tunnel.unwrap();
                assert_eq!(ssh.host, "localhost");
                assert_eq!(ssh.username, "sshuser");

                // The auth field should be default (empty password) after roundtrip
                match &ssh.auth {
                    SshAuth::Password { password } => {
                        assert_eq!(password, "", "Password should be empty after deserialization");
                    }
                    _ => panic!("Expected Password auth"),
                }
            }
            _ => panic!("Expected PostgreSQL config"),
        }
    }
}
