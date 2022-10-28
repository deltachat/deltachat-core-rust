ARC-Authentication-Results: i=1; mx.google.com;
       dkim=pass header.i=@aol.com header.s=a2048 header.b=aox1b6+y;
       spf=pass (google.com: domain of alice@aol.com designates 87.248.110.84 as permitted sender) smtp.mailfrom=alice@aol.com;
       dmarc=pass (p=REJECT sp=REJECT dis=NONE) header.from=aol.com
Authentication-Results: mx.google.com;
       dkim=pass header.i=@aol.com header.s=a2048 header.b=aox1b6+y;
       spf=pass (google.com: domain of alice@aol.com designates 87.248.110.84 as permitted sender) smtp.mailfrom=alice@aol.com;
       dmarc=pass (p=REJECT sp=REJECT dis=NONE) header.from=aol.com
From: <alice@aol.com>
To: <alice@gmail.com>
