import * as anchor from "@project-serum/anchor";
import { Keypair, PublicKey, LAMPORTS_PER_SOL, Transaction, VersionedTransaction, MessageV0, TransactionMessage, SystemProgram } from "@solana/web3.js"
import { Program } from "@project-serum/anchor";
import { TokenVestingProgram } from "../target/types/token_vesting_program";
import moment from "moment";
import { expect } from "chai";

const stepAmount =  moment().add(1, 'day');
const ONE_DAY_IN_SECONDS = stepAmount.diff(moment(), 'seconds');

function delay(ms: number) {
  return new Promise( resolve => setTimeout(resolve, ms) );
}

interface Params {
  cliffSeconds: anchor.BN
  durationSeconds: anchor.BN,
  secondsPerSlice: anchor.BN,
  startUnix: anchor.BN,
  grantTokenAmount: anchor.BN,
}

interface UnitTestParams {
    employeeKeypair: Keypair;
    accounts: {
        employer: PublicKey;
        employee: PublicKey;
        grant: PublicKey;
        grantCustody: PublicKey;
    }
}

export function makeParams(startTime: string, cliffMonths: number, vestingYears: number, stepFunctionSeconds: number, grantTokenAmountInSol: string ): Params {
    const grantTokenAmount = new anchor.BN(grantTokenAmountInSol).mul(new anchor.BN(LAMPORTS_PER_SOL));
    const current = moment(startTime, 'YYYY-MM-DD');
    const vestingCliff = current.clone().add(cliffMonths, 'months');
    const vestingDuration = current.clone().add(vestingYears, 'years');
    return {
        startUnix: new anchor.BN(current.unix()),
        durationSeconds: new anchor.BN(vestingDuration.unix()),
        cliffSeconds: new anchor.BN(vestingCliff.unix()),
        grantTokenAmount,
        secondsPerSlice: new anchor.BN(stepFunctionSeconds),
    }
}

describe("Initialize", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const { connection } = provider;
    const program = anchor.workspace.TokenVestingProgram as Program<TokenVestingProgram>;

    let unitTestParams: UnitTestParams;
    beforeEach(async () => {
        const employeeKeypair = Keypair.generate();

        const employer = provider.wallet.publicKey;
        const employee = employeeKeypair.publicKey;
        const [grant,] = await PublicKey.findProgramAddress(
            [
                Buffer.from("grant_account"),
                provider.wallet.publicKey.toBuffer(),
                employee.toBuffer(),
            ],
            program.programId
        );
        const [grantCustody,] = await PublicKey.findProgramAddress(
            [
                Buffer.from("grant_custody"),
                provider.wallet.publicKey.toBuffer(),
                employee.toBuffer(),
            ],
            program.programId
        );
        unitTestParams = {
            employeeKeypair,
            accounts: {
                employee,
                employer,
                grant,
                grantCustody
            }
        };
    })

    it('correctly initializes a new account and transfers the funds', async () => {
        const params = makeParams('2020-01-01', 6, 4, ONE_DAY_IN_SECONDS, '10');
        const initializeTransaction = await program.methods
            .initialize(params)
            .accounts(unitTestParams.accounts)
            .rpc({ commitment: "confirmed" });
        console.log(`[Initialize] ${initializeTransaction}`);

        const tx = await connection.getParsedTransaction(
          initializeTransaction,
          { commitment: "confirmed" }
        );
        const rent = await connection.getMinimumBalanceForRentExemption(0);
        const totalTransfer = params.grantTokenAmount.add(new anchor.BN(rent)).toString()
        
        // Ensure that inner transfer succeded.
        const grantCustodyAcctIdx =
          tx.transaction.message.accountKeys.findIndex(
            (acct) =>
              acct.pubkey.toBase58() ===
              unitTestParams.accounts.grantCustody.toBase58()
          );
        const grantCustodyDelta = tx.meta.postBalances[grantCustodyAcctIdx] - tx.meta.preBalances[grantCustodyAcctIdx]
        expect(grantCustodyDelta.toString()).to.eql(totalTransfer);
        const transferIx = tx.meta.innerInstructions[0].instructions.find(ix => (ix as any).parsed.type === "transfer");
        expect((transferIx as any).parsed.info).eql({
            source: unitTestParams.accounts.employer.toBase58(),
            destination: unitTestParams.accounts.grantCustody.toBase58(),
            lamports: parseInt(totalTransfer),
        });

        // Check account
        const foo = await program.account.grant.fetch(unitTestParams.accounts.grant);
        expect(foo.employee.toBase58()).to.eq(unitTestParams.accounts.employee.toBase58());
        expect(foo.employer.toBase58()).to.eq(unitTestParams.accounts.employer.toBase58());
        expect(foo.initialized).to.eq(true);
        expect(foo.revoked).to.eq(false);
        expect(foo.alreadyIssuedTokenAmount.toNumber()).to.eq(0);
        expect(foo.params.cliffSeconds.toNumber()).to.eq(params.cliffSeconds.toNumber());
        expect(foo.params.startUnix.toNumber()).to.eq(params.startUnix.toNumber());
        expect(foo.params.durationSeconds.toNumber()).to.eq(params.durationSeconds.toNumber());
        expect(foo.params.grantTokenAmount.toNumber()).to.eq(params.grantTokenAmount.toNumber());
        expect(foo.params.secondsPerSlice.toNumber()).to.eq(params.secondsPerSlice.toNumber());
    });

    // Reverts if funds are unavailable
});