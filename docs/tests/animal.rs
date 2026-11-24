/// @brief Represents the dietary classification of an animal.
pub enum Diet {
    /// Eats only plants.
    Herbivore,
    /// Eats only meat.
    Carnivore,
    /// Eats both plants and meat.
    Omnivore,
}

/// @brief Abstract base class representing a generic animal.
///
/// Subclasses must implement `make_sound`.
///
/// @example
/// struct Dog;
/// impl Speakable for Dog {
///     fn make_sound(&self) { println!("Woof!"); }
/// }
pub struct Animal {
    name: String,
    age: u32,
    diet: Diet,
}

impl Animal {
    /// @brief Constructs a new Animal object.
    /// @param[in] name The animal's name.
    /// @param[in] age The animal's age.
    /// @param[in] diet The animal's diet.
    pub fn new(name: String, age: u32, diet: Diet) -> Self {
        Self { name, age, diet }
    }

    /// @brief Produces the characteristic sound of the animal.
    pub fn make_sound(&self) {
        todo!()
    }

    /// @brief Retrieves the animal's name.
    /// @return A reference to the name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// @brief Anything that can produce a characteristic sound.
pub trait Speakable {
    /// @brief Emit the sound.
    fn speak(&self);
}
