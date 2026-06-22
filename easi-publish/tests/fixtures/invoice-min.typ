// Minimal but representative invoice fixture for tests + benches.
// Data is supplied as a virtual JSON file (see JsonFile / the bench).
#let data = json("invoice.json")

#set page(paper: "a4", margin: 2cm)
#set text(size: 11pt)

= Invoice #data.invoice_number
Date: #data.date

#table(
  columns: 3,
  [*Description*], [*Qty*], [*Amount*],
  ..data.items.map(it => ([#it.description], [#it.quantity], [#it.amount])).flatten(),
)

#align(right)[*Total:* #data.total]
