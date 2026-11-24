# animal.py

## `Diet` Enum Reference

Represents the dietary classification of an animal.

### Enumerators

| Enumerator | Description |
|------------|-------------|
| `Herbivore` |  |
| `Carnivore` |  |
| `Omnivore` |  |

---

## `Animal` Class Reference *abstract*

Abstract base class representing a generic animal.

Subclasses must implement @c make_sound. The base class stores the animal's name, age and diet, exposed through @c get_name.

### Example

```
class Dog(Animal):
    def __init__(self) -> None:
        super().__init__("Rex", 3, Diet.Carnivore)

    def make_sound(self) -> None:
        print("Woof!")

rex = Dog()
rex.make_sound()   # Woof!
assert rex.get_name() == "Rex"
```

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `` | `def __init__(name: str, age: int, diet: Diet)`<br>Constructs a new Animal object. |
| `` | `def __del__()`<br>Virtual destructor to ensure proper cleanup of derived classes. |
| `None` | abstract `def make_sound() -> None`<br>Produces the characteristic sound of the animal. |
| `str` | `def get_name() -> str`<br>Retrieves the animal's name. |

### Method Contracts

#### `__init__()`

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `name` | `str` | The animal's name. |
| [in] | `age` | `int` | The animal's age in years. |
| [in] | `diet` | `Diet` | The animal's diet. |

#### `make_sound()`

Concrete subclasses must override this method. The implementation should emit a single vocalization via @c print.

#### Example

```
class Cat(Animal):
    def __init__(self) -> None:
        super().__init__("Whiskers", 2, Diet.Carnivore)

    def make_sound(self) -> None:
        print("Meow!")

Cat().make_sound()   # Meow!
```

#### `get_name()`

**Returns:** The name passed to the constructor.

#### Example

```
class Dog(Animal):
    def make_sound(self) -> None:
        print("Woof!")

assert Dog().get_name() == "Rex"
```
