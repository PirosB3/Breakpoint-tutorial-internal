import * as anchor from "@project-serum/anchor";
import {
  Keypair,
  PublicKey,
  LAMPORTS_PER_SOL,
  Transaction,
  VersionedTransaction,
  MessageV0,
  TransactionMessage,
  SystemProgram,
  ConfirmOptions,
  Finality,
} from "@solana/web3.js";
import { Program } from "@project-serum/anchor";
import { TokenVestingProgram } from "../target/types/token_vesting_program";
import moment from "moment";
import * as spl from '@solana/spl-token';
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

interface ParsedTokenTransfer {
    amount: string,
    authority: string,
    destination: string,
    source: string
}

interface PDAAccounts {
    grant: PublicKey;
    escrowAuthority: PublicKey;
    escrowTokenAccount: PublicKey;
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

const COMMITMENT: {commitment: Finality} = {commitment: 'confirmed'};


describe("Initialize", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const { connection } = provider;
    const program = anchor.workspace.TokenVestingProgram as Program<TokenVestingProgram>;

    const createTokenAccount = async (user: anchor.web3.PublicKey, mint: anchor.web3.PublicKey, fundingAmount?: number): Promise<anchor.web3.PublicKey> => {
        const userAssociatedTokenAccount = await spl.Token.getAssociatedTokenAddress(
            spl.ASSOCIATED_TOKEN_PROGRAM_ID,
            spl.TOKEN_PROGRAM_ID,
            mint,
            user,
        )

        // Fund user with some SOL
        let txFund = new anchor.web3.Transaction();
        if (user.toBase58() !== provider.wallet.publicKey.toBase58()) {
            txFund.add(anchor.web3.SystemProgram.transfer({
                fromPubkey: provider.wallet.publicKey,
                toPubkey: user,
                lamports: 5 * anchor.web3.LAMPORTS_PER_SOL,
            }));
        }
        txFund.add(spl.Token.createAssociatedTokenAccountInstruction(
            spl.ASSOCIATED_TOKEN_PROGRAM_ID,
            spl.TOKEN_PROGRAM_ID,
            mint,
            userAssociatedTokenAccount,
            user,
            provider.wallet.publicKey,
        ))
        if (fundingAmount !== undefined) {
            txFund.add(spl.Token.createMintToInstruction(
                spl.TOKEN_PROGRAM_ID,
                mint,
                userAssociatedTokenAccount,
                provider.wallet.publicKey,
                [],
                fundingAmount,
            ));
        }

        const txFundTokenSig = await provider.sendAndConfirm(txFund, [], COMMITMENT);
        console.log(JSON.stringify(await connection.getParsedAccountInfo(userAssociatedTokenAccount)));
        console.log(`[${userAssociatedTokenAccount.toBase58()}] New associated account for mint ${mint.toBase58()}: ${txFundTokenSig}`);
        return userAssociatedTokenAccount;
    }

    const createMint = async (): Promise<anchor.web3.PublicKey> => {
        const tokenMint = new anchor.web3.Keypair();
        const lamportsForMint = await provider.connection.getMinimumBalanceForRentExemption(spl.MintLayout.span);
        let tx = new anchor.web3.Transaction();

        // Allocate mint
        tx.add(
            anchor.web3.SystemProgram.createAccount({
                programId: spl.TOKEN_PROGRAM_ID,
                space: spl.MintLayout.span,
                fromPubkey: provider.wallet.publicKey,
                newAccountPubkey: tokenMint.publicKey,
                lamports: lamportsForMint,
            })
        )
        // Allocate wallet account
        tx.add(
            spl.Token.createInitMintInstruction(
                spl.TOKEN_PROGRAM_ID,
                tokenMint.publicKey,
                9,
                provider.wallet.publicKey,
                provider.wallet.publicKey,
            )
        );
        const signature = await provider.sendAndConfirm(tx, [tokenMint], COMMITMENT);

        console.log(`[${tokenMint.publicKey}] Created new mint account at ${signature}`);
        return tokenMint.publicKey;
    }

    const getPDAs = async (params: {employee: PublicKey, employer: PublicKey}): Promise<PDAAccounts> => {
        const [grant,] = await PublicKey.findProgramAddress(
            [
                Buffer.from("grant"),
                params.employer.toBuffer(),
                params.employee.toBuffer(),
            ],
            program.programId
        );
        const [escrowAuthority,] = await PublicKey.findProgramAddress(
            [
                Buffer.from("authority"),
                grant.toBuffer(),
            ],
            program.programId
        );
        const [escrowTokenAccount,] = await PublicKey.findProgramAddress(
            [
                Buffer.from("tokens"),
                grant.toBuffer(),
            ],
            program.programId
        );
        return {
            grant,
            escrowAuthority,
            escrowTokenAccount,
        }
    }

    beforeEach(async () => {
        const employeeKeypair = Keypair.generate();
    })

    it('correctly initializes a new account and transfers the funds', async () => {
        const employer = provider.wallet.publicKey;
        const employee = Keypair.generate().publicKey;
        const { grant, escrowAuthority, escrowTokenAccount } = await getPDAs({employer, employee});
        const mint = await createMint();
        const employerFundingAccount = await createTokenAccount(provider.wallet.publicKey, mint, 100_000 * LAMPORTS_PER_SOL);

        const params = makeParams('2020-01-01', 6, 4, ONE_DAY_IN_SECONDS, '10');
        console.log(mint.toBase58());
        const initializeTransaction = await program.methods
            .initialize(params)
            .accounts({
                employee,
                employer,
                grant,
                escrowAuthority,
                escrowTokenAccount,
                mint,
                employerFundingAccount,
            })
            .rpc(COMMITMENT);
        console.log(`[Initialize] ${initializeTransaction}`);

        const tx = await connection.getParsedTransaction(
          initializeTransaction,
          COMMITMENT,
        );
        // console.log(JSON.stringify(tx));
        const totalTransfer = params.grantTokenAmount;
        
        // Ensure that inner transfer succeded.
        const transferIx: any = tx.meta.innerInstructions[0].instructions.find(
          ix => (ix as any).parsed.type === "transfer" && ix.programId.toBase58() == spl.TOKEN_PROGRAM_ID.toBase58()
        );
        const parsedInfo: ParsedTokenTransfer = transferIx.parsed.info;
        expect(parsedInfo).eql({
            amount: params.grantTokenAmount.toString(),
            authority: employer.toBase58(),
            destination: escrowTokenAccount.toBase58(),
            source: employerFundingAccount.toBase58()
        });

        // // Check data in the 
        const grantData = await program.account.grant.fetch(grant);
        expect(grantData.employee.toBase58()).to.eq(employee.toBase58());
        expect(grantData.employer.toBase58()).to.eq(employer.toBase58());
        expect(grantData.initialized).to.eq(true);
        expect(grantData.revoked).to.eq(false);
        expect(grantData.alreadyIssuedTokenAmount.toNumber()).to.eq(0);
        expect(grantData.params.cliffSeconds.toNumber()).to.eq(params.cliffSeconds.toNumber());
        expect(grantData.params.startUnix.toNumber()).to.eq(params.startUnix.toNumber());
        expect(grantData.params.durationSeconds.toNumber()).to.eq(params.durationSeconds.toNumber());
        expect(grantData.params.grantTokenAmount.toNumber()).to.eq(params.grantTokenAmount.toNumber());
        expect(grantData.params.secondsPerSlice.toNumber()).to.eq(params.secondsPerSlice.toNumber());
        expect(grantData.mint.toBase58()).to.eql(mint.toBase58())
        expect(grantData.employer.toBase58()).to.eql(employer.toBase58())
        expect(grantData.employee.toBase58()).to.eql(employee.toBase58())
        expect(grantData.bumps.grant).to.not.eql(0);
        expect(grantData.bumps.escrowAuthority).to.not.eql(0);
        expect(grantData.bumps.escrowTokenAccount).to.not.eql(0);
        expect(grantData.grantCustodyBump).to.not.eql(0);
    });

    // Reverts if funds are unavailable
});