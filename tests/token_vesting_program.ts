import * as anchor from "@project-serum/anchor";
import { Keypair, PublicKey, LAMPORTS_PER_SOL, Transaction, VersionedTransaction, MessageV0, TransactionMessage, SystemProgram } from "@solana/web3.js"
import { Program } from "@project-serum/anchor";
import { TokenVestingProgram } from "../target/types/token_vesting_program";

function delay(ms: number) {
  return new Promise( resolve => setTimeout(resolve, ms) );
}

describe("token_vesting_program", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TokenVestingProgram as Program<TokenVestingProgram>;

  it("Is initialized!", async () => {
    // Add your test here.
    const employee = Keypair.generate();
    console.log(`[ME] ${provider.wallet.publicKey.toBase58()}`);
    console.log(`[Program] ${program.programId.toBase58()}`);
    console.log(`[EMPLOYEE] ${employee.publicKey.toBase58()}`);

    const [grant,] = await PublicKey.findProgramAddress(
      [
        Buffer.from("grant_account"),
        provider.wallet.publicKey.toBuffer(),
        employee.publicKey.toBuffer(),
      ],
      program.programId
    );

    const [grantCustody,] = await PublicKey.findProgramAddress(
      [
        Buffer.from("grant_custody"),
        provider.wallet.publicKey.toBuffer(),
        employee.publicKey.toBuffer(),
      ],
      program.programId
    );
    console.log(`[GRANT ACCT] ${grant.toBase58()}`);
    console.log(`[GRANT CUSTODY] ${grantCustody.toBase58()}`);

    const tx = await program.methods.initialize({
      cliffSeconds: new anchor.BN("31560000"),
      durationSeconds: new anchor.BN("126240000"),
      secondsPerSlice: new anchor.BN("1"),
      startUnix: new anchor.BN("1585181904"),
      grantTokenAmount: new anchor.BN("100000000"),
    }).accounts({
      employer: provider.wallet.publicKey,
      employee: employee.publicKey,
      grant,
      grantCustody,
    })
    .rpc({commitment: 'confirmed'});

    const data = await provider.connection.getParsedTransaction(tx, {commitment: 'confirmed'});
    console.log(JSON.stringify(data));

    await delay(8000);


    // Revoke
    const txRevoke = await program.methods
      .revoke()
      .accounts({
        employer: provider.wallet.publicKey,
        employee: employee.publicKey,
        grantAccount,
        grantCustody,
      })
      .rpc({ commitment: "confirmed" });
    
    console.log(txRevoke);
    const datatxRevoke = await provider.connection.getParsedTransaction(txRevoke, {commitment: 'confirmed'});
    console.log(JSON.stringify(datatxRevoke));
    
    const grantAcctK = await program.account.grant.fetch(grantAccount);
    console.log(JSON.stringify(grantAcctK));
    return


    // Withdraw
    const tx2 = await program.methods
      .withdraw([new anchor.BN("3")] )
      .accounts({
        employer: provider.wallet.publicKey,
        employee: employee.publicKey,
        grantAccount,
        grantCustody,
      })
      .signers([employee])
      .rpc({ commitment: "confirmed" });
    const data2 = await provider.connection.getParsedTransaction(tx2, {commitment: 'confirmed'});
    console.log(JSON.stringify(data2));

    const grantAcct = await program.account.grant.fetch(grantAccount);
    console.log(JSON.stringify(grantAcct));

    const providerAcct = await provider.connection.getAccountInfo(employee.publicKey);
    console.log(providerAcct);

    const tx3 = await program.methods
      .withdraw([new anchor.BN("9")] )
      .accounts({
        employer: provider.wallet.publicKey,
        employee: employee.publicKey,
        grantAccount,
        grantCustody,
      })
      .signers([employee])
      .rpc({commitment: 'confirmed'})
    const data3 = await provider.connection.getParsedTransaction(tx3, {commitment: 'confirmed'});
    console.log(JSON.stringify(data3));

    // const {blockhash } = await provider.connection.getLatestBlockhash();

    // let tx2 = new Transaction({
    //   feePayer: provider.wallet.publicKey,
    //   recentBlockhash: blockhash,
    // }).add(
    //       SystemProgram.transfer({
    //         fromPubkey: provider.wallet.publicKey,
    //         toPubkey: employee.publicKey,
    //         lamports: 1000000000,
    //       })
    // );
    // tx2 = await provider.wallet.signTransaction(tx2);
    // const res = await provider.sendAndConfirm(tx2);
    // console.log(res);

    // const tx2 = await program.methods.initialize({
    //   cliffSeconds: new anchor.BN("31560000"),
    //   durationSeconds: new anchor.BN("126240000"),
    //   secondsPerSlice: new anchor.BN("2592000"),
    //   startUnix: new anchor.BN("1585181904"),
    //   grantTokenAmount: new anchor.BN("30000000"),
    // }).accounts({
    //   employer: provider.wallet.publicKey,
    //   employee: employee.publicKey,
    //   grantAccount,
    //   grantCustody,
    // })
    // .rpc({commitment: 'confirmed'});
  });
});
