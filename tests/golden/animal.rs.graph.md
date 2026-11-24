# Contract Graph

## Class Diagram

```mermaid
classDiagram
  class Diet {
    <<enumeration>>
    Herbivore
    Carnivore
    Omnivore
  }
  class Animal {
    -String name
    -u32 age
    -Diet diet
    +new()
    +make_sound()
    +name()
  }
  class Speakable {
    <<interface>>
    +speak()
  }
  Animal ..> Diet
```

## Inheritance Diagram

```mermaid
classDiagram
  class Animal {
  }
  class Speakable {
    <<interface>>
  }
```

## Collaboration Diagram

```mermaid
flowchart LR
  Diet["Diet"]
  Animal["Animal"]
  Speakable["Speakable"]
  Animal -->|uses| Diet
```

## Call Graph

```mermaid
flowchart LR
  fn_animal_Animal_new["animal.Animal.new"]
  fn_animal_Animal_make_sound["animal.Animal.make_sound"]
  fn_animal_Animal_name["animal.Animal.name"]
  fn_animal_Speakable_speak["animal.Speakable.speak"]
```

## Caller Graph

```mermaid
flowchart LR
  fn_animal_Animal_new["animal.Animal.new"]
  fn_animal_Animal_make_sound["animal.Animal.make_sound"]
  fn_animal_Animal_name["animal.Animal.name"]
  fn_animal_Speakable_speak["animal.Speakable.speak"]
```

## Include Graph

```mermaid
flowchart LR
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_animal_rs["animal.rs"]
```

## Directory Diagram

```mermaid
flowchart LR
  dir_0["/home/phng/Code/Wheels/Dozen/tests/fixtures"]
```
