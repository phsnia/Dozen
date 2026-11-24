# Contract Graph

## Class Diagram

```mermaid
classDiagram
  class Shape {
    <<interface>>
    +area()
    +describe()
    +name()
  }
  class Rectangle {
    +f64 width
    -f64 height
    -Option~f64~ cached
    +new()
    +scale()
    +recalculate()
    +area()
    +name()
    +add()
  }
  class Kind {
    <<enumeration>>
    Circle
    Rectangle
    Polygon
  }
  Shape <|.. Rectangle
```

## Inheritance Diagram

```mermaid
classDiagram
  class Shape {
    <<interface>>
  }
  class Rectangle {
  }
  Shape <|.. Rectangle
```

## Collaboration Diagram

```mermaid
flowchart LR
  Shape["Shape"]
  Rectangle["Rectangle"]
  Kind["Kind"]
```

## Call Graph

```mermaid
flowchart LR
  fn_shapes_Shape_area["shapes.Shape.area"]
  fn_shapes_Shape_describe["shapes.Shape.describe"]
  fn_shapes_Shape_name["shapes.Shape.name"]
  fn_shapes_Rectangle_new["shapes.Rectangle.new"]
  fn_shapes_Rectangle_scale["shapes.Rectangle.scale"]
  fn_shapes_Rectangle_recalculate["shapes.Rectangle.recalculate"]
  fn_shapes_Rectangle_area["shapes.Rectangle.area"]
  fn_shapes_Rectangle_name["shapes.Rectangle.name"]
  fn_shapes_Rectangle_add["shapes.Rectangle.add"]
  fn_shapes_Rectangle_add -->|calls| fn_shapes_Rectangle_new
```

## Caller Graph

```mermaid
flowchart LR
  fn_shapes_Shape_area["shapes.Shape.area"]
  fn_shapes_Shape_describe["shapes.Shape.describe"]
  fn_shapes_Shape_name["shapes.Shape.name"]
  fn_shapes_Rectangle_new["shapes.Rectangle.new"]
  fn_shapes_Rectangle_scale["shapes.Rectangle.scale"]
  fn_shapes_Rectangle_recalculate["shapes.Rectangle.recalculate"]
  fn_shapes_Rectangle_area["shapes.Rectangle.area"]
  fn_shapes_Rectangle_name["shapes.Rectangle.name"]
  fn_shapes_Rectangle_add["shapes.Rectangle.add"]
  fn_shapes_Rectangle_new -->|called by| fn_shapes_Rectangle_add
```

## Include Graph

```mermaid
flowchart LR
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_shapes_rs["shapes.rs"]
  ext_0["std::fmt::Debug"]
  ext_1["std::ops::Add"]
  ext_2["utils"]
  ext_3["num_traits"]
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_shapes_rs --> ext_0
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_shapes_rs --> ext_1
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_shapes_rs --> ext_2
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_shapes_rs --> ext_3
```

## Directory Diagram

```mermaid
flowchart LR
  dir_0["/home/phng/Code/Wheels/Dozen/tests/fixtures"]
```
