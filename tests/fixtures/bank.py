"""Banking primitives.

@details Demonstrates accounts and transfers.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from decimal import Decimal

from .errors import InsufficientFunds


@dataclass
class Account:
    """A single bank account.

    @note Accounts start empty.
    @warning Not thread safe.
    @see transaction
    """

    owner: str
    balance: Decimal = field(default_factory=Decimal)

    def __init__(self, owner: str, limit: int = 0):
        """Create an account.

        @param[in] owner The owner's name.
        @param[in] limit Overdraft limit in cents.
        """
        self.owner = owner
        self._limit: int = limit
        self.__secret = "pin"
        self.history = owner

    def deposit(self, amount: Decimal) -> Decimal:
        """Add funds.

        @param[in] amount Money to add.
        @return The new balance.
        @throws ValueError When amount is negative.
        @example
        acc.deposit(Decimal("5"))
        """
        if amount < 0:
            raise ValueError("negative")
        self.balance += amount
        return self.balance

    def _audit(self) -> None:
        """Internal audit."""
        print(self.__secret)

    def withdraw(self, amount: Decimal) -> Decimal:
        """Remove funds.

        @details Checks the limit first.
        @param[in] amount Money to remove.
        @return The new balance.
        @throws InsufficientFunds When funds are short.
        @note Transfers are atomic.
        @warning Never call from an interrupt handler.
        @see deposit
        """
        if amount > self.balance:
            raise InsufficientFunds(self.owner)
        return self.balance - amount


class SavingsAccount(Account):
    """An account that earns interest."""

    rate: float

    def accrue(self) -> float:
        """Compute interest.

        @return Interest accrued.
        """
        return float(self.balance) * self.rate


async def transfer(source: Account, target: Account, amount: Decimal) -> None:
    """Move money between accounts.

    @param[in] source Account to debit.
    @param[in] target Account to credit.
    @param[in] amount Money to move.
    @throws ValueError When amount is not positive.
    @note Uses synchronous calls.
    @warning Both accounts must be writable.
    @see Account.withdraw
    """
    if amount <= 0:
        raise ValueError("non-positive")
    source.withdraw(amount)
    target.deposit(amount)
