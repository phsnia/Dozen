# Contract Graph

## Class Diagram

```mermaid
classDiagram
  class Account {
    +str owner
    +Decimal balance
    #int _limit
    -__secret
    +str history
    +__init__()
    +deposit()
    +withdraw()
  }
  class SavingsAccount {
    +float rate
    +accrue()
  }
  Account <|-- SavingsAccount
```

## Inheritance Diagram

```mermaid
classDiagram
  class Account {
  }
  class SavingsAccount {
  }
  Account <|-- SavingsAccount
```

## Collaboration Diagram

```mermaid
flowchart LR
  Account["Account"]
  SavingsAccount["SavingsAccount"]
```

## Call Graph

```mermaid
flowchart LR
  fn_bank_Account___init__["bank.Account.__init__"]
  fn_bank_Account_deposit["bank.Account.deposit"]
  fn_bank_Account_withdraw["bank.Account.withdraw"]
  fn_bank_SavingsAccount_accrue["bank.SavingsAccount.accrue"]
  fn_bank_transfer["bank.transfer"]
  fn_bank_transfer -->|calls| fn_bank_Account_withdraw
  fn_bank_transfer -->|calls| fn_bank_Account_deposit
```

## Caller Graph

```mermaid
flowchart LR
  fn_bank_Account___init__["bank.Account.__init__"]
  fn_bank_Account_deposit["bank.Account.deposit"]
  fn_bank_Account_withdraw["bank.Account.withdraw"]
  fn_bank_SavingsAccount_accrue["bank.SavingsAccount.accrue"]
  fn_bank_transfer["bank.transfer"]
  fn_bank_Account_withdraw -->|called by| fn_bank_transfer
  fn_bank_Account_deposit -->|called by| fn_bank_transfer
```

## Include Graph

```mermaid
flowchart LR
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_bank_py["bank.py"]
  ext_0["__future__"]
  ext_1["dataclasses"]
  ext_2["decimal"]
  ext_3[".errors"]
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_bank_py --> ext_0
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_bank_py --> ext_1
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_bank_py --> ext_2
  file__home_phng_Code_Wheels_Dozen_tests_fixtures_bank_py --> ext_3
```

## Directory Diagram

```mermaid
flowchart LR
  dir_0["/home/phng/Code/Wheels/Dozen/tests/fixtures"]
```
