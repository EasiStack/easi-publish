// Invoice Template
// Data is loaded from invoice.json at compile time.

#let data = json("invoice.json")

#let accent = rgb("#2563eb")
#let light-gray = rgb("#f3f4f6")
#let border-gray = rgb("#e5e7eb")
#let text-muted = rgb("#6b7280")

#set page(
  paper: "a4",
  margin: (top: 2cm, bottom: 2cm, left: 2cm, right: 2cm),
)

#set text(font: ("Inter", "Libertinus Serif"), size: 10pt)

// --- Render target ---
// PDF lays the document out natively. HTML embeds the *same* laid-out frame as
// inline SVG (via `html.frame`) so it matches the PDF — Typst's default HTML
// export is semantic and drops the paged layout (page, grids, table fills, …).
// 17cm = A4 width (21cm) minus the 2cm side margins.
#show: doc => context if target() == "html" {
  html.frame(block(width: 17cm, doc))
} else {
  doc
}

// --- Header ---

#grid(
  columns: (1fr, 1fr),
  align: (left, right),
  [
    #text(size: 28pt, weight: "bold", fill: accent)[INVOICE]
    #v(4pt)
    #text(fill: text-muted)[#data.invoice_number]
  ],
  [],
)

#v(16pt)

// --- From / To ---

#let address-block(label, person) = {
  text(size: 8pt, weight: "bold", fill: text-muted)[#label]
  v(4pt)
  text(weight: "bold", person.name)
  linebreak()
  person.address.join([\
  ])
  if "email" in person {
    linebreak()
    person.email
  }
  if "phone" in person {
    linebreak()
    person.phone
  }
  if "vat" in person {
    linebreak()
    text(fill: text-muted, "VAT: " + person.vat)
  }
}

#grid(
  columns: (1fr, 1fr),
  column-gutter: 24pt,
  address-block("FROM", data.from),
  address-block("BILL TO", data.to),
)

#v(12pt)

// --- Dates ---

#grid(
  columns: (1fr, 1fr, 1fr),
  [
    #text(size: 8pt, weight: "bold", fill: text-muted)[DATE ISSUED]
    #v(4pt)
    #data.date_issued
  ],
  [
    #text(size: 8pt, weight: "bold", fill: text-muted)[DUE DATE]
    #v(4pt)
    #data.date_due
  ],
  if "currency" in data [
    #text(size: 8pt, weight: "bold", fill: text-muted)[CURRENCY]
    #v(4pt)
    #data.currency
  ],
)

#v(20pt)

// --- Line Items Table ---

#let cur = if "currency_symbol" in data { data.currency_symbol } else { "$" }

#table(
  columns: (auto, 1fr, auto, auto, auto),
  align: (center, left, right, right, right),
  inset: (x: 10pt, y: 8pt),
  stroke: none,
  fill: (_, y) => if y == 0 { accent } else if calc.even(y) { light-gray },

  // Header
  table.header(
    text(fill: white, weight: "bold", "\#"),
    text(fill: white, weight: "bold", "Description"),
    text(fill: white, weight: "bold", "Qty"),
    text(fill: white, weight: "bold", "Unit Price"),
    text(fill: white, weight: "bold", "Amount"),
  ),

  // Line items
  ..data.items.enumerate().map(((i, item)) => {
    let desc = if "detail" in item {
      [#text(weight: 500)[#item.description] \ #text(size: 8pt, fill: text-muted)[#item.detail]]
    } else {
      text(weight: 500)[#item.description]
    }
    (
      str(i + 1),
      desc,
      str(item.quantity),
      cur + item.unit_price,
      cur + item.amount,
    )
  }).flatten(),
)

#v(8pt)

// --- Totals ---

#{
  let rows = ()
  rows.push(text(fill: text-muted, "Subtotal"))
  rows.push(cur + data.subtotal)

  if "discount" in data and data.discount != none {
    rows.push(text(fill: text-muted, "Discount"))
    rows.push("- " + cur + data.discount)
  }

  if "tax_label" in data and data.tax_label != none {
    rows.push(text(fill: text-muted, data.tax_label))
    rows.push(cur + data.tax_amount)
  }

  align(right,
    box(width: 250pt)[
      #grid(
        columns: (1fr, auto),
        row-gutter: 6pt,
        inset: (x: 10pt, y: 4pt),
        ..rows,
      )

      #line(length: 100%, stroke: 0.5pt + border-gray)

      #grid(
        columns: (1fr, auto),
        inset: (x: 10pt, y: 6pt),
        text(size: 14pt, weight: "bold")[Total Due],
        text(size: 14pt, weight: "bold", fill: accent, cur + data.total),
      )
    ]
  )
}

#v(24pt)

// --- Payment Details ---

#if "payment" in data {
  line(length: 100%, stroke: 0.5pt + border-gray)
  v(12pt)
  text(size: 8pt, weight: "bold", fill: text-muted, "PAYMENT DETAILS")
  v(4pt)

  let payment-rows = data.payment.pairs().map(((key, val)) => (
    text(fill: text-muted, key),
    val,
  )).flatten()

  grid(
    columns: (auto, 1fr),
    column-gutter: 16pt,
    row-gutter: 4pt,
    ..payment-rows,
  )
}

#v(16pt)

// --- Notes ---

#if "notes" in data and data.notes != none {
  line(length: 100%, stroke: 0.5pt + border-gray)
  v(12pt)
  text(size: 8pt, weight: "bold", fill: text-muted, "NOTES")
  v(4pt)
  text(fill: text-muted, data.notes)
}
