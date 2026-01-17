import express, { Request, Response } from 'express';

const app = express();
app.use(express.json());

const PORT = process.env.PORT || 3000;
const JSONSTORE_URL = process.env.JSONSTORE_URL || 'http://jsonstore:3001';

// Hello endpoint - uses the actual helloFunction logic
app.get('/v1/api/hello/:name', async (req: Request, res: Response) => {
  const { name } = req.params;

  // Call jsonStore to store the greeting (via HTTP since it's out of process)
  try {
    const storeResponse = await fetch(`${JSONSTORE_URL}/store/greeted`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ when: Date.now(), name })
    });

    if (!storeResponse.ok) {
      console.error('Failed to store greeting:', await storeResponse.text());
    }
  } catch (error) {
    console.error('Error calling jsonStore:', error);
  }

  res.json({ message: `Hello, ${name}!` });
});

app.listen(PORT, () => {
  console.log(`API server running on http://localhost:${PORT}`);
  console.log(`JsonStore URL: ${JSONSTORE_URL}`);
});
