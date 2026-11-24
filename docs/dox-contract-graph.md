# Contract Graph

## Class Diagram

```mermaid
classDiagram
  class Config {
    +str glob
    +Path output
    +Path diagram
    +str encoding
    +Path base_dir
    +defaults()
    +load()
  }
  class EntityKind {
    <<enumeration>>
    MODULE
    FUNCTION
    METHOD
    CLASS
    STRUCT
    ENUM
    TRAIT
    INTERFACE
  }
  class Param {
    +str name
    +str type
    +str direction
    +str description
  }
  class Variant {
    +str name
    +str description
  }
  class Field {
    +str name
    +str type
    +str visibility
    +bool static
    +str description
  }
  class Doc {
    +str brief
    +str details
    +list~Param~ params
    +str returns
    +list~tuple~str, str~~ throws
    +list~str~ see
    +list~str~ notes
    +list~str~ warnings
    +list~str~ examples
  }
  class Entity {
    +EntityKind kind
    +str name
    +str signature
    +str return_type
    +Doc doc
    +bool abstract
    +list~Entity~ members
    +list~Variant~ variants
    +list~Field~ fields
    +list~str~ bases
    +list~str~ implements
    +str module
    +str path
    +int line
    +list~str~ calls
    +list~str~ includes
  }
  class DocParser {
    +parse()
  }
  class Language {
    <<abstract>>
    +tuple~str, ...~ extensions
    +parse()
    +consume_block()
  }
  class PythonLanguage {
    +parse()
  }
  class RustLanguage {
    +parse()
  }
  class CppLanguage {
    +parse()
  }
  class ReferenceRenderer {
    +render()
  }
  class MermaidRenderer {
    +render()
    +render_all()
    +class_diagram()
    +inheritance_diagram()
    +collaboration_diagram()
    +call_graph()
    +caller_graph()
    +include_graph()
    +directory_graph()
  }
  class Converter {
    +Config cfg
    +renderer
    +__init__()
    +entities()
    +file()
    +project()
    +diagram()
    +files()
  }
  class Scaffolder {
    +Path target
    +bool force
    +__init__()
    +run()
  }
  Doc ..> Param
  Entity ..> EntityKind
  Entity ..> Doc
  Entity ..> Variant
  Entity ..> Field
  Language <|-- PythonLanguage
  Language <|-- RustLanguage
  Language <|-- CppLanguage
  Converter ..> Config
```

## Inheritance Diagram

```mermaid
classDiagram
  class Config {
  }
  class Param {
  }
  class Variant {
  }
  class Field {
  }
  class Doc {
  }
  class Entity {
  }
  class DocParser {
  }
  class Language {
    <<abstract>>
  }
  class PythonLanguage {
  }
  class RustLanguage {
  }
  class CppLanguage {
  }
  class ReferenceRenderer {
  }
  class MermaidRenderer {
  }
  class Converter {
  }
  class Scaffolder {
  }
  Language <|-- PythonLanguage
  Language <|-- RustLanguage
  Language <|-- CppLanguage
```

## Collaboration Diagram

```mermaid
flowchart LR
  Config["Config"]
  EntityKind["EntityKind"]
  Param["Param"]
  Variant["Variant"]
  Field["Field"]
  Doc["Doc"]
  Entity["Entity"]
  DocParser["DocParser"]
  Language["Language"]
  PythonLanguage["PythonLanguage"]
  RustLanguage["RustLanguage"]
  CppLanguage["CppLanguage"]
  ReferenceRenderer["ReferenceRenderer"]
  MermaidRenderer["MermaidRenderer"]
  Converter["Converter"]
  Scaffolder["Scaffolder"]
  Doc -->|uses| Param
  Entity -->|uses| EntityKind
  Entity -->|uses| Doc
  Entity -->|uses| Variant
  Entity -->|uses| Field
  DocParser -->|uses| Doc
  Language -->|uses| Entity
  PythonLanguage -->|uses| Entity
  RustLanguage -->|uses| Entity
  CppLanguage -->|uses| Entity
  MermaidRenderer -->|uses| Entity
  Converter -->|uses| Config
  Converter -->|uses| Entity
```

## Call Graph

```mermaid
flowchart LR
  fn_dox_Config_defaults["dox.Config.defaults"]
  fn_dox_Config_load["dox.Config.load"]
  fn_dox_parse_config["dox.parse_config"]
  fn_dox_DocParser_parse["dox.DocParser.parse"]
  fn_dox_Language_parse["dox.Language.parse"]
  fn_dox_Language_consume_block["dox.Language.consume_block"]
  fn_dox_PythonLanguage_parse["dox.PythonLanguage.parse"]
  fn_dox_RustLanguage_parse["dox.RustLanguage.parse"]
  fn_dox_CppLanguage_parse["dox.CppLanguage.parse"]
  fn_dox_ReferenceRenderer_render["dox.ReferenceRenderer.render"]
  fn_dox_MermaidRenderer_render["dox.MermaidRenderer.render"]
  fn_dox_MermaidRenderer_render_all["dox.MermaidRenderer.render_all"]
  fn_dox_MermaidRenderer_class_diagram["dox.MermaidRenderer.class_diagram"]
  fn_dox_MermaidRenderer_inheritance_diagram["dox.MermaidRenderer.inheritance_diagram"]
  fn_dox_MermaidRenderer_collaboration_diagram["dox.MermaidRenderer.collaboration_diagram"]
  fn_dox_MermaidRenderer_call_graph["dox.MermaidRenderer.call_graph"]
  fn_dox_MermaidRenderer_caller_graph["dox.MermaidRenderer.caller_graph"]
  fn_dox_MermaidRenderer_include_graph["dox.MermaidRenderer.include_graph"]
  fn_dox_MermaidRenderer_directory_graph["dox.MermaidRenderer.directory_graph"]
  fn_dox_Converter___init__["dox.Converter.__init__"]
  fn_dox_Converter_entities["dox.Converter.entities"]
  fn_dox_Converter_file["dox.Converter.file"]
  fn_dox_Converter_project["dox.Converter.project"]
  fn_dox_Converter_diagram["dox.Converter.diagram"]
  fn_dox_Converter_files["dox.Converter.files"]
  fn_dox_Scaffolder___init__["dox.Scaffolder.__init__"]
  fn_dox_Scaffolder_run["dox.Scaffolder.run"]
  fn_dox_build_parser["dox.build_parser"]
  fn_dox__candidate_bases["dox._candidate_bases"]
  fn_dox_collect_files["dox.collect_files"]
  fn_dox__resolve_output["dox._resolve_output"]
  fn_dox__select_files["dox._select_files"]
  fn_dox_cmd_convert["dox.cmd_convert"]
  fn_dox_cmd_diagram["dox.cmd_diagram"]
  fn_dox_cmd_init["dox.cmd_init"]
  fn_dox_main["dox.main"]
  fn_dox_Config_defaults -->|calls| fn_dox_parse_config
  fn_dox_Config_load -->|calls| fn_dox_Config_defaults
  fn_dox_Config_load -->|calls| fn_dox_parse_config
  fn_dox_RustLanguage_parse -->|calls| fn_dox_Language_consume_block
  fn_dox_CppLanguage_parse -->|calls| fn_dox_Language_consume_block
  fn_dox_MermaidRenderer_render_all -->|calls| fn_dox_MermaidRenderer_render
  fn_dox_Converter_file -->|calls| fn_dox_Converter_entities
  fn_dox_Converter_project -->|calls| fn_dox_Converter_entities
  fn_dox_Converter_diagram -->|calls| fn_dox_MermaidRenderer_render_all
  fn_dox_Converter_diagram -->|calls| fn_dox_Converter_project
  fn_dox_Converter_files -->|calls| fn_dox_Converter_file
  fn_dox_collect_files -->|calls| fn_dox__candidate_bases
  fn_dox__select_files -->|calls| fn_dox_collect_files
  fn_dox_cmd_convert -->|calls| fn_dox_Config_load
  fn_dox_cmd_convert -->|calls| fn_dox__select_files
  fn_dox_cmd_convert -->|calls| fn_dox__resolve_output
  fn_dox_cmd_convert -->|calls| fn_dox_Converter_files
  fn_dox_cmd_diagram -->|calls| fn_dox_Config_load
  fn_dox_cmd_diagram -->|calls| fn_dox__select_files
  fn_dox_cmd_diagram -->|calls| fn_dox__resolve_output
  fn_dox_cmd_diagram -->|calls| fn_dox_Converter_diagram
  fn_dox_cmd_init -->|calls| fn_dox_Scaffolder_run
  fn_dox_main -->|calls| fn_dox_build_parser
```

## Caller Graph

```mermaid
flowchart LR
  fn_dox_Config_defaults["dox.Config.defaults"]
  fn_dox_Config_load["dox.Config.load"]
  fn_dox_parse_config["dox.parse_config"]
  fn_dox_DocParser_parse["dox.DocParser.parse"]
  fn_dox_Language_parse["dox.Language.parse"]
  fn_dox_Language_consume_block["dox.Language.consume_block"]
  fn_dox_PythonLanguage_parse["dox.PythonLanguage.parse"]
  fn_dox_RustLanguage_parse["dox.RustLanguage.parse"]
  fn_dox_CppLanguage_parse["dox.CppLanguage.parse"]
  fn_dox_ReferenceRenderer_render["dox.ReferenceRenderer.render"]
  fn_dox_MermaidRenderer_render["dox.MermaidRenderer.render"]
  fn_dox_MermaidRenderer_render_all["dox.MermaidRenderer.render_all"]
  fn_dox_MermaidRenderer_class_diagram["dox.MermaidRenderer.class_diagram"]
  fn_dox_MermaidRenderer_inheritance_diagram["dox.MermaidRenderer.inheritance_diagram"]
  fn_dox_MermaidRenderer_collaboration_diagram["dox.MermaidRenderer.collaboration_diagram"]
  fn_dox_MermaidRenderer_call_graph["dox.MermaidRenderer.call_graph"]
  fn_dox_MermaidRenderer_caller_graph["dox.MermaidRenderer.caller_graph"]
  fn_dox_MermaidRenderer_include_graph["dox.MermaidRenderer.include_graph"]
  fn_dox_MermaidRenderer_directory_graph["dox.MermaidRenderer.directory_graph"]
  fn_dox_Converter___init__["dox.Converter.__init__"]
  fn_dox_Converter_entities["dox.Converter.entities"]
  fn_dox_Converter_file["dox.Converter.file"]
  fn_dox_Converter_project["dox.Converter.project"]
  fn_dox_Converter_diagram["dox.Converter.diagram"]
  fn_dox_Converter_files["dox.Converter.files"]
  fn_dox_Scaffolder___init__["dox.Scaffolder.__init__"]
  fn_dox_Scaffolder_run["dox.Scaffolder.run"]
  fn_dox_build_parser["dox.build_parser"]
  fn_dox__candidate_bases["dox._candidate_bases"]
  fn_dox_collect_files["dox.collect_files"]
  fn_dox__resolve_output["dox._resolve_output"]
  fn_dox__select_files["dox._select_files"]
  fn_dox_cmd_convert["dox.cmd_convert"]
  fn_dox_cmd_diagram["dox.cmd_diagram"]
  fn_dox_cmd_init["dox.cmd_init"]
  fn_dox_main["dox.main"]
  fn_dox_parse_config -->|called by| fn_dox_Config_defaults
  fn_dox_Config_defaults -->|called by| fn_dox_Config_load
  fn_dox_parse_config -->|called by| fn_dox_Config_load
  fn_dox_Language_consume_block -->|called by| fn_dox_RustLanguage_parse
  fn_dox_Language_consume_block -->|called by| fn_dox_CppLanguage_parse
  fn_dox_MermaidRenderer_render -->|called by| fn_dox_MermaidRenderer_render_all
  fn_dox_Converter_entities -->|called by| fn_dox_Converter_file
  fn_dox_Converter_entities -->|called by| fn_dox_Converter_project
  fn_dox_MermaidRenderer_render_all -->|called by| fn_dox_Converter_diagram
  fn_dox_Converter_project -->|called by| fn_dox_Converter_diagram
  fn_dox_Converter_file -->|called by| fn_dox_Converter_files
  fn_dox__candidate_bases -->|called by| fn_dox_collect_files
  fn_dox_collect_files -->|called by| fn_dox__select_files
  fn_dox_Config_load -->|called by| fn_dox_cmd_convert
  fn_dox__select_files -->|called by| fn_dox_cmd_convert
  fn_dox__resolve_output -->|called by| fn_dox_cmd_convert
  fn_dox_Converter_files -->|called by| fn_dox_cmd_convert
  fn_dox_Config_load -->|called by| fn_dox_cmd_diagram
  fn_dox__select_files -->|called by| fn_dox_cmd_diagram
  fn_dox__resolve_output -->|called by| fn_dox_cmd_diagram
  fn_dox_Converter_diagram -->|called by| fn_dox_cmd_diagram
  fn_dox_Scaffolder_run -->|called by| fn_dox_cmd_init
  fn_dox_build_parser -->|called by| fn_dox_main
```

## Include Graph

```mermaid
flowchart LR
  file__home_phng_Code_Wheels_Dozen_docs_dox_py["dox.py"]
  ext_0["__future__"]
  ext_1["argparse"]
  ext_2["ast"]
  ext_3["glob"]
  ext_4["re"]
  ext_5["sys"]
  ext_6["abc"]
  ext_7["dataclasses"]
  ext_8["enum"]
  ext_9["pathlib"]
  ext_10["typing"]
  file__home_phng_Code_Wheels_Dozen_docs_dox_py --> ext_0
  file__home_phng_Code_Wheels_Dozen_docs_dox_py --> ext_1
  file__home_phng_Code_Wheels_Dozen_docs_dox_py --> ext_2
  file__home_phng_Code_Wheels_Dozen_docs_dox_py --> ext_3
  file__home_phng_Code_Wheels_Dozen_docs_dox_py --> ext_4
  file__home_phng_Code_Wheels_Dozen_docs_dox_py --> ext_5
  file__home_phng_Code_Wheels_Dozen_docs_dox_py --> ext_6
  file__home_phng_Code_Wheels_Dozen_docs_dox_py --> ext_7
  file__home_phng_Code_Wheels_Dozen_docs_dox_py --> ext_8
  file__home_phng_Code_Wheels_Dozen_docs_dox_py --> ext_9
  file__home_phng_Code_Wheels_Dozen_docs_dox_py --> ext_10
```

## Directory Diagram

```mermaid
flowchart LR
  dir_0["/home/phng/Code/Wheels/Dozen/docs"]
```
