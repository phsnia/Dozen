# @brief Animal hierarchy shell helpers.
#
# @details Demonstrates Bash functions, calls, and comments.

# @brief Produce the characteristic animal sound.
# @return The sound text.
make_sound() {
    echo "woof"
}

# @brief Retrieve the animal name.
# @param[in] name Animal name.
get_name() {
    echo "$1"
}

# @brief Feed the animal.
# @param[in] amount Portion size in grams.
feed() {
    make_sound
    echo "fed $1 grams"
}
