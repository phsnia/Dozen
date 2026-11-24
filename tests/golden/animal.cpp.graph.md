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
    <<abstract>>
    #std::string name_
    #int age_
    -Diet diet_
    -int count_$
    +make_sound()
    +name()
    +feed()
  }
  class Dog {
    +make_sound()
  }
  Animal ..> Diet
  Animal <|-- Dog
```

## Inheritance Diagram

```mermaid
classDiagram
  class Animal {
    <<abstract>>
  }
  class Dog {
  }
  Animal <|-- Dog
```

## Collaboration Diagram

```mermaid
flowchart LR
  Diet["Diet"]
  Animal["Animal"]
  Dog["Dog"]
  Animal -->|uses| Diet
```

## Call Graph

```mermaid
flowchart LR
  fn_animal_Animal__name["animal.Animal::name"]
  fn_animal_Animal__feed["animal.Animal::feed"]
  fn_animal_Dog__make_sound["animal.Dog::make_sound"]
  fn_animal_Animal_make_sound["animal.Animal.make_sound"]
  fn_animal_Animal_name["animal.Animal.name"]
  fn_animal_Animal_feed["animal.Animal.feed"]
  fn_animal_Dog_make_sound["animal.Dog.make_sound"]
```

## Caller Graph

```mermaid
flowchart LR
  fn_animal_Animal__name["animal.Animal::name"]
  fn_animal_Animal__feed["animal.Animal::feed"]
  fn_animal_Dog__make_sound["animal.Dog::make_sound"]
  fn_animal_Animal_make_sound["animal.Animal.make_sound"]
  fn_animal_Animal_name["animal.Animal.name"]
  fn_animal_Animal_feed["animal.Animal.feed"]
  fn_animal_Dog_make_sound["animal.Dog.make_sound"]
```

## Include Graph

```mermaid
flowchart LR
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_animal_cpp["animal.cpp"]
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_animal_hpp["animal.hpp"]
  ext_0["string"]
  ext_1["vector"]
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_animal_cpp --> file__home_phng_Code_Wheels_Dozen_tests_fixtures_animal_hpp
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_animal_hpp --> ext_0
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_animal_hpp --> ext_1
```

## Directory Diagram

```mermaid
flowchart LR
  dir_0["/home/phng/Code/Wheels/Dozen/tests/fixtures"]
```
