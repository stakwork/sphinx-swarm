import { parseDate, shortTransactionId } from "./";

export function parseLndInvoices(transactions) {
  const parsedInvoices = transactions.invoices;
  if (parsedInvoices.length > 0) {
    let trans = [];
    for (let i = 0; i < parsedInvoices.length; i++) {
      const invoice = parsedInvoices[i];
      trans.push({
        id: invoice.payment_request,
        index: `${i + 1}.`,
        invoice: shortTransactionId(invoice.payment_request),
        date: parseDate(invoice.settle_date),
        amount: invoice.amt_paid_sat,
      });
    }
    return trans;
  } else {
    return [];
  }
}

export function parseLndPayments(transactions) {
  const parseTransactions = transactions.payments;
  if (parseTransactions.length > 0) {
    let trans = [];
    for (let i = 0; i < parseTransactions.length; i++) {
      const tran = parseTransactions[i];
      trans.push({
        id: tran.payment_request,
        index: `${i + 1}.`,
        invoice: shortTransactionId(tran.payment_request),
        date: parseDate(tran.creation_date),
        amount: tran.value_sat,
      });
    }
    return trans;
  } else {
    return [];
  }
}
