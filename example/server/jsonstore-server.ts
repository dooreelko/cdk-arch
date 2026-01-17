import express, { Request, Response } from 'express';
import { Pool } from 'pg';

const app = express();
app.use(express.json());

const PORT = process.env.PORT || 3001;

// Postgres connection
const pool = new Pool({
  host: process.env.POSTGRES_HOST || 'postgres',
  port: parseInt(process.env.POSTGRES_PORT || '5432'),
  database: process.env.POSTGRES_DB || 'jsonstore',
  user: process.env.POSTGRES_USER || 'postgres',
  password: process.env.POSTGRES_PASSWORD || 'postgres'
});

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

// Store endpoint - POST /store/{collection}
app.post('/store/:collection', async (req: Request, res: Response) => {
  const { collection } = req.params;
  const document = req.body;

  try {
    await pool.query(
      'INSERT INTO documents (collection, data) VALUES ($1, $2)',
      [collection, JSON.stringify(document)]
    );
    res.json({ success: true });
  } catch (error) {
    console.error('Error storing document:', error);
    res.status(500).json({ success: false, error: 'Failed to store document' });
  }
});

// Get endpoint - GET /get/{collection}
app.get('/get/:collection', async (req: Request, res: Response) => {
  const { collection } = req.params;

  try {
    const result = await pool.query(
      'SELECT data FROM documents WHERE collection = $1 ORDER BY created_at',
      [collection]
    );
    res.json(result.rows.map(row => row.data));
  } catch (error) {
    console.error('Error getting documents:', error);
    res.status(500).json({ error: 'Failed to get documents' });
  }
});

// Start server after DB init
initDb().then(() => {
  app.listen(PORT, () => {
    console.log(`JsonStore server running on http://localhost:${PORT}`);
  });
}).catch(err => {
  console.error('Failed to initialize database:', err);
  process.exit(1);
});
