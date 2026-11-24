/// @file Animal hierarchy implementation.

#include "animal.hpp"

namespace zoo {

/// @brief Construct an animal.
/// @param[in] name The animal's name.
/// @param[in] age The animal's age in years.
/// @param[in] diet The animal's diet.
Animal::Animal(std::string name, int age, Diet diet) {
    name_ = name;
    age_ = age;
    diet_ = diet;
}

/// @brief Retrieve the animal's name.
/// @return The stored name.
std::string Animal::name() const {
    return name_;
}

/// @brief Feed the animal.
/// @param[in] amount Portion size in grams.
void Animal::feed(int amount) {
    apply_diet(amount);
    growl();
}

/// @brief Construct a dog.
Dog::Dog(std::string name) {
    name_ = name;
}

/// @brief Produce a bark.
void Dog::make_sound() const {
    bark(2);
}

}  // namespace zoo
