import * as anchor from "@project-serum/anchor";
import {
  PublicKey,
  LAMPORTS_PER_SOL,
  Finality,
} from "@solana/web3.js";
import { ASSOCIATED_TOKEN_PROGRAM_ID, MintLayout, Token, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import moment, { isMoment, Moment } from "moment";


export const stepAmount =  moment().add(1, 'day');
export const ONE_DAY_IN_SECONDS = stepAmount.diff(moment(), 'seconds');

export interface Params {
    cliffSeconds: anchor.BN
    durationSeconds: anchor.BN,
    secondsPerSlice: anchor.BN,
    startUnix: anchor.BN,
    grantTokenAmount: anchor.BN,
  }
  
export interface ParsedTokenTransfer {
      amount: string,
      authority: string,
      destination: string,
      source: string
  }
  
export interface PDAAccounts {
      grant: PublicKey;
      escrowAuthority: PublicKey;
      escrowTokenAccount: PublicKey;
  }
  
export function makeParams(startTime: string | Moment, cliffMonths: number, vestingYears: number, stepFunctionSeconds: number, grantTokenAmountInSol: string ): Params {
      const current = isMoment(startTime) ? startTime : moment(startTime, 'YYYY-MM-DD');
      const grantTokenAmount = new anchor.BN(grantTokenAmountInSol).mul(new anchor.BN(LAMPORTS_PER_SOL));
      const vestingCliff = current.clone().add(cliffMonths, 'months').diff(current, 'seconds')
      const vestingDuration = current.clone().add(vestingYears, 'years').diff(current, 'seconds');
      return {
          startUnix: new anchor.BN(current.unix()),
          durationSeconds: new anchor.BN(vestingDuration),
          cliffSeconds: new anchor.BN(vestingCliff),
          grantTokenAmount,
          secondsPerSlice: new anchor.BN(stepFunctionSeconds),
      }
  }
  
export const COMMITMENT: {commitment: Finality} = {commitment: 'confirmed'};

export const createTokenAccount = async (provider: anchor.AnchorProvider, user: anchor.web3.PublicKey, mint: anchor.web3.PublicKey, fundingAmount?: number): Promise<anchor.web3.PublicKey> => {
    const userAssociatedTokenAccount = await Token.getAssociatedTokenAddress(
        ASSOCIATED_TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
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
    txFund.add(Token.createAssociatedTokenAccountInstruction(
        ASSOCIATED_TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        mint,
        userAssociatedTokenAccount,
        user,
        provider.wallet.publicKey,
    ))
    if (fundingAmount !== undefined) {
        txFund.add(Token.createMintToInstruction(
            TOKEN_PROGRAM_ID,
            mint,
            userAssociatedTokenAccount,
            provider.wallet.publicKey,
            [],
            fundingAmount,
        ));
    }

    const txFundTokenSig = await provider.sendAndConfirm(txFund, [], COMMITMENT);
    console.log(`[${userAssociatedTokenAccount.toBase58()}] New associated account for mint ${mint.toBase58()}: ${txFundTokenSig}`);
    return userAssociatedTokenAccount;
}

export const createMint = async (provider: anchor.AnchorProvider): Promise<anchor.web3.PublicKey> => {
    const wallet = provider.wallet;
    const tokenMint = new anchor.web3.Keypair();
    const lamportsForMint = await provider.connection.getMinimumBalanceForRentExemption(MintLayout.span);
    let tx = new anchor.web3.Transaction();

    // Allocate mint
    tx.add(
        anchor.web3.SystemProgram.createAccount({
            programId: TOKEN_PROGRAM_ID,
            space: MintLayout.span,
            fromPubkey: wallet.publicKey,
            newAccountPubkey: tokenMint.publicKey,
            lamports: lamportsForMint,
        })
    )
    // Allocate wallet account
    tx.add(
        Token.createInitMintInstruction(
            TOKEN_PROGRAM_ID,
            tokenMint.publicKey,
            9,
            wallet.publicKey,
            wallet.publicKey,
        )
    );
    const signature = await provider.sendAndConfirm(tx, [tokenMint], COMMITMENT);

    console.log(`[${tokenMint.publicKey}] Created new mint account at ${signature}`);
    return tokenMint.publicKey;
}

export const getPDAs = async (params: {programId: PublicKey, employee: PublicKey, employer: PublicKey}): Promise<PDAAccounts> => {
    const [grant,] = await PublicKey.findProgramAddress(
        [
            Buffer.from("grant"),
            params.employer.toBuffer(),
            params.employee.toBuffer(),
        ],
        params.programId
    );
    const [escrowAuthority,] = await PublicKey.findProgramAddress(
        [
            Buffer.from("authority"),
            grant.toBuffer(),
        ],
        params.programId
    );
    const [escrowTokenAccount,] = await PublicKey.findProgramAddress(
        [
            Buffer.from("tokens"),
            grant.toBuffer(),
        ],
        params.programId
    );
    return {
        grant,
        escrowAuthority,
        escrowTokenAccount,
    }
}