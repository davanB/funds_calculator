# Client Funds Calculator

## Correctness
The main logic resides in the client module. It updates a client, given a new transaction.
That was the area I decided to put my unit testing focus as the other modules are either support or have to do with reading/writing the client data as csv.

One improvement I would make in the future as I couldnt figure out how to do it with Serde is instead of deserialzing the csv into a regular struct I would use **Enums** with typed fields.
This is because in the case of disputes, resolves and chargebacks there is no concept of amount. Although we can model this as an `Option<f32>` I think we can do better by
removing the concept of amount altogether. This would remove any need of checking for `Some(amount)`. The enum type would better match the concept of each transaction type.

Although this could have been done by an extra processing step. I realized this too late.
csv -> deserialize with serde into struct -> convert into list of enums with typed fields.

Although we dont use the amount if incorrectly provided by a partner, its incorrect (and potentially dangerous) to have it populated.

## Efficiency
There was the question if we can stream values, if the csv was very large or if streamed over TCP.
The main concern is having to keep transactions if a dispute occured.
Thinking of real life banking, you can only dispute within a time frame. This would allow transactions to be dropped from history as new ones arrived.


Thank you for reading!