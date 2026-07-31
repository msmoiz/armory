use std::cmp::Ordering;

/// Sorts version strings in ascending order.
pub fn sort_versions(a: &String, b: &String) -> Ordering {
    let mut a = a.split(".");
    let mut b = b.split(".");
    loop {
        match (a.next(), b.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_b)) => return Ordering::Less,
            (Some(_a), None) => return Ordering::Greater,
            (Some(a), Some(b)) => {
                let a = a.parse::<u8>().expect("version part should be integer");
                let b = b.parse::<u8>().expect("version part should be integer");

                if a < b {
                    return Ordering::Less;
                }

                if a > b {
                    return Ordering::Greater;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::sort_versions;

    #[test]
    fn version_sort() {
        let mut versions = vec![
            String::from("1.0"),
            String::from("2.0.1"),
            String::from("1.10.0"),
            String::from("1.9.0"),
            String::from("1.0.0"),
            String::from("2"),
        ];

        versions.sort_by(sort_versions);

        assert_eq!(
            versions,
            [
                String::from("1.0"),
                String::from("1.0.0"),
                String::from("1.9.0"),
                String::from("1.10.0"),
                String::from("2"),
                String::from("2.0.1"),
            ]
        )
    }
}
