import express from 'express';
import { Pool } from 'pg';
import { DockerApiServer, StorageAdapter, architectureBinding } from 'cdk-arch';
import { jsonStore } from '../src/architecture';

// Postgres connection
const pool = new Pool({
  host: process.env.POSTGRES_HOST || 'postgres',
  port: parseInt(process.env.POSTGRES_PORT || '5432'),
  database: process.env.POSTGRES_DB || 'jsonstore',
  user: process.env.POSTGRES_USER || 'postgres',
  password: process.env.POSTGRES_PASSWORD || 'postgres'
});

// Postgres storage adapter
const postgresStorage: StorageAdapter = {
  async store(collection: string, document: any): Promise<{ success: boolean }> {
    await pool.query(
      'INSERT INTO documents (collection, data) VALUES ($1, $2)',
      [collection, JSON.stringify(document)]
    );
    return { success: true };
  },

  async get(collection: string): Promise<any[]> {
    const result = await pool.query(
      'SELECT data FROM documents WHERE collection = $1 ORDER BY created_at',
      [collection]
    );
    return result.rows.map(row => row.data);
  }
};

// Initialize database table with retry logic
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
  throw new Error('Failed to connect to database after retries');
}

// Bind jsonStore to its endpoint
architectureBinding.bind(jsonStore, {
  host: 'jsonstore',
  port: parseInt(process.env.PORT || '3001')
});

// Create server using DockerApiServer with Postgres storage
const server = new DockerApiServer(jsonStore, { binding: architectureBinding });
const app = server.createApp(express, postgresStorage);

const PORT = process.env.PORT || 3001;

// Start server after DB init
initDb().then(() => {
  app.listen(PORT, () => {
    console.log(`JsonStore server running on port ${PORT}`);
  });
}).catch(err => {
  console.error('Failed to initialize database:', err);
  process.exit(1);
});
