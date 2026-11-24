# animal.cpp

## `Animal::name()`

```
std::string Animal::name()
```

Retrieve the animal's name.

### Returns

The stored name.

---

## `Animal::feed()`

```
void Animal::feed(int amount)
```

Feed the animal.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `amount` | `int` | Portion size in grams. |

---

## `Dog::make_sound()`

```
void Dog::make_sound()
```

Produce a bark.

# animal.hpp

## `Diet` Enum Reference

Kinds of animal diet.

### Enumerators

| Enumerator | Description |
|------------|-------------|
| `Herbivore` |  |
| `Carnivore` |  |
| `Omnivore` |  |

---

## `Animal` Class Reference *abstract*

Abstract base animal. Stores a name, an age, and a diet.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| protected | `std::string` | `name_` | @brief The animal's name. |
| protected | `int` | `age_` | @brief The animal's age in years. |
| private | `Diet` | `diet_` | @brief The animal's diet. |
| static private | `int` | `count_` | @brief Number of live animals. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `void` | abstract `void make_sound()`<br>Produce the animal's sound. |
| `std::string` | `std::string name()`<br>Retrieve the animal's name. |
| `void` | `void feed(int amount)`<br>Feed the animal. |

### Method Contracts

#### `make_sound()`

**Returns:** Nothing.

#### `name()`

**Returns:** The stored name.

#### `feed()`

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `amount` | `` | Portion size in grams. |

---

## `Dog` Class Reference

A domestic dog.

**Inherits:** `Animal`

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `void` | `void make_sound()`<br>Produce a bark. |
