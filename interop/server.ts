import express from 'express'
import { PrivateKey, ProtoWallet } from '@bsv/sdk'
import { createAuthMiddleware } from '@reference/auth'
import knexFactory from 'knex'
import { bindMessageBoxRuntime } from '@reference/runtime'
import { postAuth } from '@reference/routes'
const main = async () => {
const db = knexFactory({client:'better-sqlite3',connection:{filename:':memory:'},useNullAsDefault:true,pool:{min:1,max:1},postProcessResponse(result){
 const convert=(row:any)=>{if(row==null||typeof row!=='object')return row;for(const k of ['created_at','updated_at','last_used','createdAt','updatedAt','lastUsed'])if(row[k]!=null)row[k]=new Date(row[k]);return row};return Array.isArray(result)?result.map(convert):convert(result)
}})
await db.schema.createTable('messageBox',t=>{t.increments('messageBoxId');t.timestamps(true,true);t.string('type').notNullable();t.string('identityKey').notNullable();t.unique(['type','identityKey'])})
await db.schema.createTable('messages',t=>{t.string('messageId').primary();t.timestamps(true,true);t.integer('messageBoxId').notNullable();t.string('sender').notNullable();t.string('recipient').notNullable();t.text('body').notNullable();t.timestamp('expires_at').nullable()})
await db.schema.createTable('message_permissions',t=>{t.increments('id');t.timestamps(true,true);t.string('recipient').notNullable();t.string('sender').nullable();t.string('sender_scope').notNullable().defaultTo('');t.string('message_box').notNullable();t.integer('recipient_fee').notNullable();t.unique(['recipient','message_box','sender_scope'])})
await db.schema.createTable('server_fees',t=>{t.increments('id');t.timestamps(true,true);t.string('message_box').unique();t.integer('delivery_fee')})
await db.schema.createTable('message_resource_locks',t=>{t.string('identity_key').primary();t.timestamp('updated_at').notNullable()})
await db.schema.createTable('device_registrations',t=>{t.increments('id');t.timestamps(true,true);t.string('identity_key').notNullable();t.string('fcm_token').unique();t.string('device_id').nullable();t.string('platform').nullable();t.timestamp('last_used').nullable();t.boolean('active').defaultTo(true)})
const wallet=new ProtoWallet(PrivateKey.fromRandom());bindMessageBoxRuntime({knex:db,wallet})
const app=express();app.use(express.json({limit:'5mb'}));app.use(createAuthMiddleware({wallet,allowUnauthenticated:false,logLevel:'error'}))
for(const route of postAuth) (app as any)[route.type](route.path,route.func)
app.listen(5002,'127.0.0.1',()=>console.log('READY real pinned routes + real BRC103/104 + in-memory SQLite on 127.0.0.1:5002'))
}
main().catch(e=>{console.error(e);process.exitCode=1})
