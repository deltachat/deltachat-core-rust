use crate::location::Location;

/* * the structure behind dc_array_t */
#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum dc_array_t {
    Locations(Vec<Location>),
    Uint(Vec<u32>),
}

impl dc_array_t {
    pub fn new(capacity: usize) -> Self {
        dc_array_t::Uint(Vec::with_capacity(capacity))
    }

    /// Constructs a new, empty `dc_array_t` holding locations with specified `capacity`.
    pub fn new_locations(capacity: usize) -> Self {
        dc_array_t::Locations(Vec::with_capacity(capacity))
    }

    pub fn add_id(&mut self, item: u32) {
        if let Self::Uint(array) = self {
            array.push(item);
        } else {
            panic!("Attempt to add id to array of other type");
        }
    }

    pub fn add_location(&mut self, location: Location) {
        if let Self::Locations(array) = self {
            array.push(location)
        } else {
            panic!("Attempt to add a location to array of other type");
        }
    }

    pub fn get_id(&self, index: usize) -> u32 {
        match self {
            Self::Locations(array) => array[index].location_id,
            Self::Uint(array) => array[index] as u32,
        }
    }

    pub fn get_location(&self, index: usize) -> &Location {
        if let Self::Locations(array) = self {
            &array[index]
        } else {
            panic!("Not an array of locations")
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Locations(array) => array.is_empty(),
            Self::Uint(array) => array.is_empty(),
        }
    }

    /// Returns the number of elements in the array.
    pub fn len(&self) -> usize {
        match self {
            Self::Locations(array) => array.len(),
            Self::Uint(array) => array.len(),
        }
    }

    pub fn clear(&mut self) {
        match self {
            Self::Locations(array) => array.clear(),
            Self::Uint(array) => array.clear(),
        }
    }

    pub fn search_id(&self, needle: u32) -> Option<usize> {
        if let Self::Uint(array) = self {
            for (i, &u) in array.iter().enumerate() {
                if u == needle {
                    return Some(i);
                }
            }
            None
        } else {
            panic!("Attempt to search for id in array of other type");
        }
    }

    pub fn sort_ids(&mut self) {
        if let dc_array_t::Uint(v) = self {
            v.sort();
        } else {
            panic!("Attempt to sort array of something other than uints");
        }
    }

    pub fn as_ptr(&self) -> *const u32 {
        if let dc_array_t::Uint(v) = self {
            v.as_ptr()
        } else {
            panic!("Attempt to convert array of something other than uints to raw");
        }
    }
}

impl From<Vec<u32>> for dc_array_t {
    fn from(array: Vec<u32>) -> Self {
        dc_array_t::Uint(array)
    }
}

impl From<Vec<Location>> for dc_array_t {
    fn from(array: Vec<Location>) -> Self {
        dc_array_t::Locations(array)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dc_array() {
        let mut arr = dc_array_t::new(7);
        assert!(arr.is_empty());

        for i in 0..1000 {
            arr.add_id(i + 2);
        }

        assert_eq!(arr.len(), 1000);

        for i in 0..1000 {
            assert_eq!(arr.get_id(i), (i + 2) as u32);
        }

        arr.clear();

        assert!(arr.is_empty());

        arr.add_id(13);
        arr.add_id(7);
        arr.add_id(666);
        arr.add_id(0);
        arr.add_id(5000);

        arr.sort_ids();

        assert_eq!(arr.get_id(0), 0);
        assert_eq!(arr.get_id(1), 7);
        assert_eq!(arr.get_id(2), 13);
        assert_eq!(arr.get_id(3), 666);
    }

    #[test]
    #[should_panic]
    fn test_dc_array_out_of_bounds() {
        let mut arr = dc_array_t::new(7);
        for i in 0..1000 {
            arr.add_id(i + 2);
        }
        arr.get_id(1000);
    }
}
