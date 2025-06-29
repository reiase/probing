#[cfg(test)]
mod tests {
    use core::f64;

    use probing_proto::types::basic::*;

    #[test]
    fn test_seq_append_from_nil() {
        let mut seq = Seq::Nil;

        // Test creating different sequence types from Nil
        assert!(seq.append(42i32).is_ok());
        assert_eq!(seq, Seq::SeqI32(vec![42]));

        let mut seq = Seq::Nil;
        assert!(seq.append("hello".to_string()).is_ok());
        assert_eq!(seq, Seq::SeqText(vec!["hello".to_string()]));

        let mut seq = Seq::Nil;
        assert!(seq.append(f64::consts::PI).is_ok());
        assert_eq!(seq, Seq::SeqF64(vec![f64::consts::PI]));
    }

    #[test]
    fn test_seq_append_type_mismatch() {
        let mut seq = Seq::SeqI32(vec![1, 2, 3]);

        // Should fail when trying to append wrong type
        assert!(seq.append("string".to_string()).is_err());
        assert_eq!(seq, Seq::SeqI32(vec![1, 2, 3])); // unchanged
    }

    #[test]
    fn test_seq_append_nil_to_nil() {
        let mut seq = Seq::Nil;
        assert!(seq.append(Ele::Nil).is_ok());
        assert_eq!(seq, Seq::Nil); // Should remain Nil
    }

    #[test]
    fn test_seq_len_and_empty() {
        let seq = Seq::Nil;
        assert_eq!(seq.len(), 0);
        assert!(seq.is_empty());

        let seq = Seq::SeqI32(vec![1, 2, 3]);
        assert_eq!(seq.len(), 3);
        assert!(!seq.is_empty());

        let seq = Seq::SeqText(vec![]);
        assert_eq!(seq.len(), 0);
        assert!(seq.is_empty());
    }
}
