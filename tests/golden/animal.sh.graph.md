# Contract Graph

## Class Diagram

```mermaid
classDiagram
```

## Inheritance Diagram

```mermaid
classDiagram
```

## Collaboration Diagram

```mermaid
flowchart LR
```

## Call Graph

```mermaid
flowchart LR
  fn_animal_make_sound["animal.make_sound"]
  fn_animal_get_name["animal.get_name"]
  fn_animal_feed["animal.feed"]
  fn_animal_feed -->|calls| fn_animal_make_sound
```

## Caller Graph

```mermaid
flowchart LR
  fn_animal_make_sound["animal.make_sound"]
  fn_animal_get_name["animal.get_name"]
  fn_animal_feed["animal.feed"]
  fn_animal_make_sound -->|called by| fn_animal_feed
```

## Include Graph

```mermaid
flowchart LR
  file__ROOT_tests_fixtures_animal_sh["animal.sh"]
```

## Directory Diagram

```mermaid
flowchart LR
  dir_0["<ROOT>/tests/fixtures"]
```
