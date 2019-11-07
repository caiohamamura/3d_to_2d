#include "stdint.h"
#include "stdio.h"

#ifdef _WIN32 
#define strncasecmp _strnicmp
#define strcasecmp _stricmp
#define _USE_MATH_DEFINES
#endif


// libLidVoxel.h
void rotateX(double *,double);
void rotateZ(double *,double);
// end libLidVoxel.h

// libLasProcess.h
float determineGaussSep(float,float);
//

/*####################################*/
/*TLS beams, polar coords*/

typedef struct{
  float zen;     /*zenith*/
  float az;      /*azimuth*/
  float x;        /*beam origin*/
  float y;        /*beam origin*/
  float z;        /*beam origin*/
  uint32_t shotN; /*shot number within this scan*/
  uint8_t nHits;  /*number of hits of this beam*/
  float *r;       /*range*/
  float *refl;    /*reflectance*/
}tlsBeam;


/*##########################################*/
/*TLS point cloud*/

typedef struct{
  int bin;       /*bin number*/
  float x;       /*coordinate*/
  float y;       /*coordinate*/
  float z;       /*coordinate*/
  float gap;     /*voxel gap fraction*/
  float r;       /*range*/
  uint16_t refl; /*reflectance*/
  uint32_t hitN;  /*hit number*/
  uint8_t nHits;  /*number of hits of this beam*/
}tlsPoint;


/*##########################################*/
/*TLS scan*/

typedef struct{
  tlsBeam *beam;     /*array of beams*/
  tlsPoint *point;   /*array of points*/
  double xOff;       /*offset to allow coords to be floats*/
  double yOff;       /*offset to allow coords to be floats*/
  double zOff;       /*offset to allow coords to be floats*/
  uint32_t nBeams;   /*number of beams in this scan*/
  uint32_t nPoints;  /*number of points in this scan*/
  FILE *ipoo;        /*file pointer*/
  uint32_t pOffset;  /*current point position for buffering*/
  uint32_t nRead;    /*number of beams to read at once*/
  uint32_t maxRead;  /*maximum number of beams we could have in a region*/
  uint64_t totSize;  /*total file size*/
  uint64_t totRead;  /*amount of file read so far*/
  float **matrix;    /*matrix needed for ptx files*/
}tlsScan;

void readTLSpolarBinary(char *namen,uint32_t place,tlsScan **scan);

tlsScan *tidyTLScans(tlsScan *,int);