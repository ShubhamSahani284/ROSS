import React, { useState, useEffect } from 'react'

import { useSubstrate, useSubstrateState } from './substrate-lib'
import { Grid, Table } from 'semantic-ui-react'

function Main(props) {
  const { api } = useSubstrateState()

  const {
    state: { keyring, currentAccount },
  } = useSubstrate()

  const [history, setHistory] = useState([])

  // Get the list of accounts we possess the private key for
  const keyringOptions = keyring.getPairs().map(account => ({
    key: account.address,
    value: account.address,
    text: account.meta.name.toUpperCase(),
    icon: 'user',
  }))

  const getTransactionHistoryFromBlocks = async () => {
    const history = []

    const currentAccountAddress = currentAccount.address

    try {
      let signedBlock = await api.rpc.chain.getBlock()

      let parentHash = signedBlock.block.header.parentHash

      while (parseInt(parentHash.toString()) > 0) {
        signedBlock.block.extrinsics
          .filter(ex => ex.method.method === 'transferAllowDeath')
          .forEach(ex => {
            const metadata = ex.toHuman()

            const signer = ex.signer.toString()
            const dest = metadata?.method?.args?.dest
            const amount = metadata?.method?.args?.value

            if (
              dest &&
              amount &&
              (signer === currentAccountAddress ||
                dest.Id === currentAccountAddress)
            ) {
              history.push({
                amount,
                destinationAccount: {
                  address: dest.Id,
                  name: keyringOptions.find(item => item.value === dest.Id)
                    ?.text,
                },
                sourceAccount: {
                  address: signer,
                  name: keyringOptions.find(item => item.value === signer)
                    ?.text,
                },
              })
            }
          })

        signedBlock = await api.rpc.chain.getBlock(parentHash)
        parentHash = signedBlock.block.header.parentHash
      }
    } catch (error) {
      console.debug('error', error)
    }

    console.debug(history)

    return history
  }

  useEffect(() => {
    if (currentAccount) {
      getTransactionHistoryFromBlocks().then(res => {
        setHistory(res)
      })
    }
  }, [currentAccount])

  return (
    <Grid.Column>
      <h1>Transaction History</h1>
      <Table celled striped size="small">
        <Table.Body>
          <Table.Row>
            <Table.Cell width={10}>
              <strong>Source</strong>
            </Table.Cell>
            <Table.Cell width={3}>
              <strong>Destination</strong>
            </Table.Cell>
            <Table.Cell width={3} textAlign="right">
              <strong>Amount</strong>
            </Table.Cell>
          </Table.Row>

          {history.map(transaction => (
            <Table.Row>
              <Table.Cell>
                <p>
                  {transaction.sourceAccount.name}{' '}
                  {transaction.sourceAccount.address ===
                    currentAccount.address && ' (Me)'}
                </p>
                <p>{transaction.sourceAccount.address}</p>
              </Table.Cell>
              <Table.Cell>
                <p>
                  {transaction.destinationAccount.name}{' '}
                  {transaction.destinationAccount.address ===
                    currentAccount.address && ' (Me)'}
                </p>
                <p>{transaction.destinationAccount.address}</p>
              </Table.Cell>
              <Table.Cell>{transaction.amount}</Table.Cell>
            </Table.Row>
          ))}
        </Table.Body>
      </Table>
    </Grid.Column>
  )
}

export function TransactionHistory(props) {
  const { api, keyring } = useSubstrateState()
  return keyring.getPairs && api.query ? <Main {...props} /> : null
}
