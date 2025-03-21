use std::convert::Infallible;
use std::fmt::Display;
use std::str::FromStr;

use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;

#[derive(Debug)]
enum Maybe<T> {
    Just(T),
    Nothing,
}

impl<T: FromStr> FromStr for Maybe<T> {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(Maybe::Nothing)
        } else {
            match s.parse() {
                Ok(v) => Ok(Maybe::Just(v)),
                Err(_) => Ok(Maybe::Nothing),
            }
        }
    }
}

impl<T: Display> Display for Maybe<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Maybe::Just(s) => write!(f, "{}", s),
            Maybe::Nothing => write!(f, ""),
        }
    }
}

#[test]
fn test_macro() {
    #[derive(Debug, EngineExtension)]
    struct TestExtension {
        /// describe managed_field_name1
        #[option(aliases = ["mfn1", "a"])]
        managed_field_name1: i32,

        /// describe managed_field_name2
        /// with multiline docstring
        #[option(name = "managed.field_name2", aliases = ["mfn2", "b"])]
        managed_field_name2: String,

        /// describe managed_field_name3
        #[option(name = "managed_field_name3")]
        managed_field_name3: Maybe<String>,

        /// this is a unmanaged field
        unmanaged_field_name: i64,
    }

    impl TestExtension {
        fn set_managed_field_name1(&mut self, value: i32) -> Result<(), EngineError> {
            self.managed_field_name1 = value;
            Ok(())
        }

        fn set_managed_field_name2(&mut self, value: String) -> Result<(), EngineError> {
            self.managed_field_name2 = value;
            Ok(())
        }

        fn set_managed_field_name3(&mut self, value: Maybe<String>) -> Result<(), EngineError> {
            self.managed_field_name3 = value;
            Ok(())
        }
    }

    let mut ext = TestExtension {
        managed_field_name1: 1,
        managed_field_name2: "a".to_string(),
        managed_field_name3: Maybe::Just("A".to_string()),
        unmanaged_field_name: 3,
    };

    assert_eq!(ext.get("managed_field_name1").unwrap(), "1".to_string());
    assert_eq!(ext.get("mfn1").unwrap(), "1".to_string());
    assert_eq!(ext.get("a").unwrap(), "1".to_string());

    assert!(ext.get("managed_field_name2").is_err());
    assert_eq!(ext.get("managed.field_name2").unwrap(), "a".to_string());
    assert_eq!(ext.get("mfn2").unwrap(), "a".to_string());
    assert_eq!(ext.get("b").unwrap(), "a".to_string());

    assert_eq!(ext.get("managed_field_name3").unwrap(), "A".to_string());

    assert_eq!(
        ext.set("managed_field_name1", "2").unwrap(),
        "1".to_string()
    );
    assert_eq!(ext.set("mfn1", "3").unwrap(), "2".to_string());
    assert_eq!(ext.set("a", "4").unwrap(), "3".to_string());

    assert!(ext.set("managed_field_name2", "error").is_err());
    assert_eq!(
        ext.set("managed.field_name2", "b").unwrap(),
        "a".to_string()
    );
    assert_eq!(ext.set("mfn2", "c").unwrap(), "b".to_string());
    assert_eq!(ext.set("b", "d").unwrap(), "c".to_string());

    assert_eq!(
        ext.set("managed_field_name3", "B").unwrap(),
        "A".to_string()
    );

    let opts = ext.options();
    assert_eq!(opts.len(), 3);
    assert_eq!(opts[0].key, "managed_field_name1");
    assert_eq!(opts[0].value, Some("4".to_string()));
    assert_eq!(opts[0].help, "describe managed_field_name1");
    assert_eq!(opts[1].key, "managed.field_name2");
    assert_eq!(opts[1].value, Some("d".to_string()));
    assert_eq!(
        opts[1].help,
        "describe managed_field_name2\nwith multiline docstring"
    );
    assert_eq!(opts[2].key, "managed_field_name3");
    assert_eq!(opts[2].value, Some("B".to_string()));
    assert_eq!(opts[2].help, "describe managed_field_name3");
}
