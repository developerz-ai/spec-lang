# 02 Program

> **Design.** Nothing here runs yet. The whole module. `Cents` is a whole number, so nothing rounds.

```slg
type Cents = Int where self >= 0;
type Account(balance: Cents = 0);
type Entry(request: Id[Request], from: Id[Account], to: Id[Account], amount: Cents);

state accounts: Map[Id[Account], Account];
state history: List[Entry];
state opening: Map[Id[Account], Cents];

fn moved(x: Id[Account], xs: List[Entry]) -> Int =
  xs.filter(it.to == x).sum(it.amount) - xs.filter(it.from == x).sum(it.amount);

@why("the same request, sent again, is the same transfer")
@idempotent(request)
@uses(accounts.write, history.write)
@guard(from in accounts and to in accounts else NoAccount)
@guard(from != to else SameAccount)
@guard(amount > 0 else NotPositive)
@guard(accounts[from].balance >= amount else Insufficient)
rule transfer(request: Id[Request], from: Id[Account], to: Id[Account], amount: Cents) = {
  accounts[from].balance = accounts[from].balance - amount;
  accounts[to].balance = accounts[to].balance + amount;
  history.push(Entry(request, from: from, to: to, amount: amount));
}

law conserved(t: Trace[transfer]) =
  t.steps.all(it.before.accounts.values.sum(y => y.balance) == it.after.accounts.values.sum(y => y.balance));

law history_explains(t: Trace[transfer], x: Id[Account]) =
  t.states.all(s => x in s.accounts and x in s.opening implies
    s.accounts[x].balance == s.opening[x] + moved(x, xs: s.history));

const left: Id[Account] = Id(1);
const right: Id[Account] = Id(2);
const r1: Id[Request] = Id(1);
const r2: Id[Request] = Id(2);

@given(accounts[left] -> Account(100), accounts[right] -> Account(5), opening[left] -> 100, opening[right] -> 5)
test two_accounts = {
  assert history.size == 0;
}

@given(two_accounts)
test moves = {
  transfer(r1, from: left, to: right, amount: 30);
  assert accounts[left].balance == 70;
  assert accounts[right].balance == 35;
}

@given(moves)
test sent_again = {
  transfer(r1, from: left, to: right, amount: 30);
  assert accounts[left].balance == 70;
  transfer(r1, from: left, to: right, amount: 31) -> fail KeyReused;
}

@given(two_accounts)
test refusals = {
  transfer(r1, from: left, to: right, amount: 101) -> fail Insufficient;
  transfer(r2, from: left, to: left, amount: 1) -> fail SameAccount;
  transfer(Id(3), from: left, to: Id(9), amount: 1) -> fail NoAccount;
  transfer(Id(4), from: left, to: right, amount: 0) -> fail NotPositive;
  assert history.size == 0;
}

@given(two_accounts)
test any_affordable_amount(n: 1..=100) = {
  transfer(r1, from: left, to: right, amount: n);
  assert accounts[left].balance == 100 - n;
}
```

`history_explains` has to start from a world where it holds. The engine should say that such a start exists and show one.
