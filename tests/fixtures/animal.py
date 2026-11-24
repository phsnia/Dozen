from abc import ABC, abstractmethod
from enum import Enum


class Diet(Enum):
    """Represents the dietary classification of an animal."""

    Herbivore = 1  # Eats only plants.
    Carnivore = 2  # Eats only meat.
    Omnivore  = 3  # Eats both plants and meat.


class Animal(ABC):
    """Abstract base class representing a generic animal.

    Subclasses must implement @c make_sound. The base class stores the
    animal's name, age and diet, exposed through @c get_name.

    @example
    class Dog(Animal):
        def __init__(self) -> None:
            super().__init__("Rex", 3, Diet.Carnivore)

        def make_sound(self) -> None:
            print("Woof!")

    rex = Dog()
    rex.make_sound()   # Woof!
    assert rex.get_name() == "Rex"
    """

    def __init__(self, name: str, age: int, diet: Diet):
        """Constructs a new Animal object.

        @param[in] name The animal's name.
        @param[in] age  The animal's age in years.
        @param[in] diet The animal's diet.
        """

    def __del__(self):
        """Virtual destructor to ensure proper cleanup of derived classes."""

    @abstractmethod
    def make_sound(self) -> None:
        """Produces the characteristic sound of the animal.

        Concrete subclasses must override this method. The implementation
        should emit a single vocalization via @c print.

        @example
        class Cat(Animal):
            def __init__(self) -> None:
                super().__init__("Whiskers", 2, Diet.Carnivore)

            def make_sound(self) -> None:
                print("Meow!")

        Cat().make_sound()   # Meow!
        """

    def get_name(self) -> str:
        """Retrieves the animal's name.

        @return The name passed to the constructor.

        @example
        class Dog(Animal):
            def make_sound(self) -> None:
                print("Woof!")

        assert Dog().get_name() == "Rex"
        """
        ...