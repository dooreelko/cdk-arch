import express from 'express';
import { DockerApiServer, architectureBinding } from 'cdk-arch';
import { api, jsonStore } from '../src/architecture';

// Bind components to their endpoints
architectureBinding.bind(jsonStore, {
  host: process.env.JSONSTORE_HOST || 'jsonstore',
  port: parseInt(process.env.JSONSTORE_PORT || '3001')
});
architectureBinding.bind(api, {
  host: 'hello-api',
  port: parseInt(process.env.PORT || '3000')
});

// Create the server from the api container definition
const server = new DockerApiServer(api, { binding: architectureBinding });
const jsonStoreClient = server.getRemoteClient(jsonStore);

// Override the hello handler to use the remote jsonStore client
const app = express();
app.use(express.json());

app.get('/v1/api/hello/:name', async (req, res) => {
  try {
    const { name } = req.params;

    // Call jsonStore via HTTP
    if (jsonStoreClient) {
      await jsonStoreClient.store('greeted', { when: Date.now(), name });
    }

    res.json({ message: `Hello, ${name}!` });
  } catch (error: any) {
    console.error('Error:', error);
    res.status(500).json({ error: error.message });
  }
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
  console.log(`API server running on port ${PORT}`);
});
