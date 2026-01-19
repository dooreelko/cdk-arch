import express from 'express';
import { Pool } from 'pg';
import { architectureBinding } from 'cdk-arch';
import { DockerApiServer } from '../docker-api-server';
import { jsonStore } from '../../architecture';

// Postgres connection
const pool = new Pool({
  host: process.env.POSTGRES_HOST || 'postgres',
  port: parseInt(process.env.POSTGRES_PORT || '5432'),
  database: process.env.POSTGRES_DB || 'jsonstore',
  user: process.env.POSTGRES_USER || 'postgres',
  password: process.env.POSTGRES_PASSWORD || 'postgres'
});

// Initialize database with retry
async function initDb(retries = 30, delay = 1000): Promise<void> {
  for (let i = 0; i < retries; i++) {
    try {
      const client = await pool.connect();
      try {
        await client.query(`
          CREATE TABLE IF NOT EXISTS documents (
            id SERIAL PRIMARY KEY,
            collection VARCHAR(255) NOT NULL,
            data JSONB NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
          )
        `);
        console.log('Database initialized');
        return;
      } finally {
        client.release();
      }
    } catch (err) {
      console.log(`Waiting for database... (${i + 1}/${retries})`);
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }
  throw new Error('Failed to connect to database');
}

// Postgres storage handlers
const postgresStore = async (collection: string, document: any): Promise<{ success: boolean }> => {
  await pool.query(
    'INSERT INTO documents (collection, data) VALUES ($1, $2)',
    [collection, JSON.stringify(document)]
  );
  return { success: true };
};

const postgresGet = async (collection: string): Promise<any[]> => {
  const result = await pool.query(
    'SELECT data FROM documents WHERE collection = $1 ORDER BY created_at',
    [collection]
  );
  return result.rows.map(row => row.data);
};

// Bind jsonStore with Postgres overloads
const PORT = parseInt(process.env.PORT || '3001');

architectureBinding.bind(jsonStore, {
  host: 'jsonstore',
  port: PORT,
  overloads: {
    storeFunction: postgresStore,
    getFunction: postgresGet
  }
});

// Start server after DB init
initDb().then(() => {
  const server = new DockerApiServer(jsonStore, { binding: architectureBinding });
  server.start(express, PORT);
}).catch(err => {
  console.error('Failed to initialize:', err);
  process.exit(1);
});
