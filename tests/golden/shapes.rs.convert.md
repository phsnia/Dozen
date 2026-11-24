# shapes.rs

## `Shape` Trait Reference

A drawable shape.

Every shape exposes its area and a human-readable name.

**Inherits:** `Debug`

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `f64` | abstract `fn area(&self) -> f64`<br>Compute the shape's area. |
| `String` | `fn describe(&self) -> String`<br>Describe the shape. |
| `String` | abstract `fn name(&self) -> String`<br>The shape's display name. |

### Method Contracts

#### `area()`

**Returns:** Area in square units.

#### `describe()`

**Returns:** A short label.

---

## `Rectangle` Struct Reference

A rectangle with sides parallel to the axes.

**Implements:** `Shape`, `Add`

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `f64` | `width` |  |
| private | `f64` | `height` |  |
| private | `Option<f64>` | `cached` | Cached area, when computed. |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `Rectangle` | `fn new(width: f64, height: f64) -> Rectangle`<br>Create a rectangle. |
| `&mut Self` | `fn scale(&mut self, factor: f64) -> &mut Self`<br>Scale both dimensions. |
| `` | `fn recalculate(&mut self)`<br>Helper that must not appear in the contract. |
| `f64` | `fn area(&self) -> f64` |
| `String` | `fn name(&self) -> String` |
| `Rectangle` | `fn add(self, other: Rectangle) -> Rectangle` |

### Method Contracts

#### `new()`

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `width` | `` | The horizontal extent. |
| [in] | `height` | `` | The vertical extent. |

**Returns:** A new rectangle.

#### `scale()`

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `factor` | `` | The scale factor. |

---

## `Kind` Enum Reference

Canonical shape kinds.

### Enumerators

| Enumerator | Description |
|------------|-------------|
| `Circle` | A circle. |
| `Rectangle` | An axis-aligned rectangle. |
| `Polygon` | An arbitrary polygon. |
