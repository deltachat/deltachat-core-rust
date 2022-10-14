ARC-Authentication-Results: i=1; mx.google.com;
       dkim=pass header.i=@yahoo.com header.s=s2048 header.b=KF9PvN1o;
       spf=pass (google.com: domain of alice@yahoo.com designates 87.248.110.84 as permitted sender) smtp.mailfrom=alice@yahoo.com;
       dmarc=pass (p=REJECT sp=REJECT dis=NONE) header.from=yahoo.com
Authentication-Results: mx.google.com;
       dkim=pass header.i=@yahoo.com header.s=s2048 header.b=KF9PvN1o;
       spf=pass (google.com: domain of alice@yahoo.com designates 87.248.110.84 as permitted sender) smtp.mailfrom=alice@yahoo.com;
       dmarc=pass (p=REJECT sp=REJECT dis=NONE) header.from=yahoo.com
From: <alice@yahoo.com>
To: <alice@gmail.com>
