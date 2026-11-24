# animal.rs

## `Diet` Enum Reference

Represents the dietary classification of an animal.

### Enumerators

| Enumerator | Description |
|------------|-------------|
| `Herbivore` | Eats only plants. |
| `Carnivore` | Eats only meat. |
| `Omnivore` | Eats both plants and meat. |

---

## `Animal` Struct Reference

Abstract base class representing a generic animal. Subclasses must implement `make_sound`.

### Example

```
struct Dog;
impl Speakable for Dog {
    fn make_sound(&self) { println!("Woof!"); }
}
```

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| private | `String` | `name` |  |
| private | `u32` | `age` |  |
| private | `Diet` | `diet` |  |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Self` | `fn new(name: String, age: u32, diet: Diet) -> Self`<br>Constructs a new Animal object. |
| `` | `fn make_sound(&self)`<br>Produces the characteristic sound of the animal. |
| `&str` | `fn name(&self) -> &str`<br>Retrieves the animal's name. |

### Method Contracts

#### `new()`

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `name` | `` | The animal's name. |
| [in] | `age` | `` | The animal's age. |
| [in] | `diet` | `` | The animal's diet. |

#### `name()`

**Returns:** A reference to the name.

---

## `Speakable` Trait Reference

Anything that can produce a characteristic sound.

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `` | abstract `fn speak(&self)`<br>Emit the sound. |
