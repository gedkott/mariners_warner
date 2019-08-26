pub fn read_rows(contents: &str) -> Vec<Vec<&str>> {
    let rows: Vec<&str> = contents.lines().collect();
    rows.into_iter()
        .map(|row: &str| row.split(',').collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(Vec::<Vec<&str>>::new(), read_rows(""));
    }

    #[test]
    fn it_works_with_complicated_examples() {
        assert_eq!(
            vec![
                vec!["header", "column_name", "wtf"],
                vec!["1", "2", "3"],
                vec!["4", "5", "6"]
            ],
            read_rows("header,column_name,wtf\r\n1,2,3\r\n4,5,6")
        );
    }
}
