/// @file Animal hierarchy declarations.

#pragma once

#include <string>
#include <vector>

namespace zoo {

/// @brief Kinds of animal diet.
enum class Diet {
    Herbivore,
    Carnivore,
    Omnivore,
};

/// @brief Abstract base animal.
///
/// Stores a name, an age, and a diet.
class Animal {
public:
    /// @brief Construct an animal.
    /// @param[in] name The animal's name.
    /// @param[in] age The animal's age in years.
    /// @param[in] diet The animal's diet.
    Animal(std::string name, int age, Diet diet);

    /// @brief Produce the animal's sound.
    /// @return Nothing.
    virtual void make_sound() const = 0;

    /// @brief Retrieve the animal's name.
    /// @return The stored name.
    std::string name() const;

    /// @brief Feed the animal.
    /// @param[in] amount Portion size in grams.
    void feed(int amount);

protected:
    /// @brief The animal's name.
    std::string name_;

    /// @brief The animal's age in years.
    int age_;

private:
    /// @brief The animal's diet.
    Diet diet_;

    /// @brief Number of live animals.
    static int count_;
};

/// @brief A domestic dog.
class Dog : public Animal {
public:
    /// @brief Construct a dog.
    Dog(std::string name);

    /// @brief Produce a bark.
    void make_sound() const;
};

}  // namespace zoo
