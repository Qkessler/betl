import pandas as pd
import os
import sys
import xlrd
from colors import fg, bg, style, escape_ansi

BANK_ACCOUNT = "Assets:Checking"
MAX_DESCRIPTION_CHARS = 70

def create_ledger_transaction(transaction):
    date_string = f"{fg.BLUE}{style.BRIGHT}{transaction.value_date.date()}"
    asterisc_string = f"{fg.RED}{style.BRIGHT}*"
    description_string = f"{fg.YELLOW}{style.BRIGHT}{transaction.description[:MAX_DESCRIPTION_CHARS]}"
    transaction_string = f"""{date_string} {asterisc_string} {description_string}
    {BANK_ACCOUNT}                    {transaction.amount}€
    """
    ledger_account = input(transaction_string) 
    transaction_string += f"{ledger_account}{style.RESET_ALL}"

    return escape_ansi(transaction_string)


def main():
    wb = xlrd.open_workbook(sys.argv[1], logfile=open(os.devnull, 'w'))
    transactions = pd.read_excel(wb)[7:]
    transactions.columns = ["op_date", "value_date", "description", "amount", "total"]

    transactions_string = '\n'.join((create_ledger_transaction(transaction)
                                     for transaction in transactions.itertuples()))

    print(transactions_string)


if __name__=="__main__":
    main()

