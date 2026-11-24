# bank.py

Banking primitives.

Demonstrates accounts and transfers.

---

## `Account` Class Reference

A single bank account.

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `str` | `owner` | The owner's name. |
| public | `Decimal` | `balance` |  |
| protected | `int` | `_limit` |  |
| private | `` | `__secret` |  |
| public | `str` | `history` |  |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `` | `def __init__(owner: str, limit: int)`<br>Create an account. |
| `Decimal` | `def deposit(amount: Decimal) -> Decimal`<br>Add funds. |
| `Decimal` | `def withdraw(amount: Decimal) -> Decimal`<br>Remove funds. |

### Method Contracts

#### `__init__()`

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `owner` | `str` | The owner's name. |
| [in] | `limit` | `int` | Overdraft limit in cents. |

#### `deposit()`

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `amount` | `Decimal` | Money to add. |

**Returns:** The new balance.

**Exceptions**

| Exception | Description |
|-----------|-------------|
| `ValueError` | When amount is negative. |

#### Example

```
acc.deposit(Decimal("5"))
```

#### `withdraw()`

Checks the limit first.

**Parameters**

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `amount` | `Decimal` | Money to remove. |

**Returns:** The new balance.

**Exceptions**

| Exception | Description |
|-----------|-------------|
| `InsufficientFunds` | When funds are short. |

**Notes**

> Transfers are atomic.

**Warnings**

> Never call from an interrupt handler.

**See Also:** `deposit`

---

## `SavingsAccount` Class Reference

An account that earns interest.

**Inherits:** `Account`

### Fields

| Visibility | Type | Name | Description |
|------------|------|------|-------------|
| public | `float` | `rate` |  |

### Public Member Functions

| Return Type | Signature & Description |
|-------------|-------------------------|
| `float` | `def accrue() -> float`<br>Compute interest. |

### Method Contracts

#### `accrue()`

**Returns:** Interest accrued.

---

## `transfer()`

```
async def transfer(source: Account, target: Account, amount: Decimal) -> None
```

Move money between accounts.

### Parameters

| Direction | Name | Type | Description |
|-----------|------|------|-------------|
| [in] | `source` | `Account` | Account to debit. |
| [in] | `target` | `Account` | Account to credit. |
| [in] | `amount` | `Decimal` | Money to move. |

### Exceptions

| Exception | Description |
|-----------|-------------|
| `ValueError` | When amount is not positive. |

### Notes

> Uses synchronous calls.

### Warnings

> Both accounts must be writable.

### See Also

- `Account.withdraw`
