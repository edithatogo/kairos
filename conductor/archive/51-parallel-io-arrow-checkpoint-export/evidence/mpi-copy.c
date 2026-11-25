/* Copyright (c) Kairos contributors. SPDX-License-Identifier: Apache-2.0 */
#include <limits.h>
#include <mpi.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void require(int condition, const char *message)
{
    if (!condition) {
        fprintf(stderr, "%s\n", message);
        MPI_Abort(MPI_COMM_WORLD, 1);
        abort();
    }
}

static void check(int status)
{
    if (status != MPI_SUCCESS) {
        int length;
        char message[MPI_MAX_ERROR_STRING];
        MPI_Error_string(status, message, &length);
        require(0, message);
    }
}

static void check_count(MPI_Status *status, int expected)
{
    int actual;
    check(MPI_Get_count(status, MPI_BYTE, &actual));
    require(actual == expected, "short collective I/O");
}

int main(int argc, char **argv)
{
    int rank, ranks;
    MPI_File input, output;
    MPI_Offset bytes;
    MPI_Status status;

    check(MPI_Init(&argc, &argv));
    check(MPI_Comm_rank(MPI_COMM_WORLD, &rank));
    check(MPI_Comm_size(MPI_COMM_WORLD, &ranks));
    require(argc == 3 && ranks == 2, "expected input, output and two ranks");
    check(MPI_File_open(MPI_COMM_WORLD, argv[1], MPI_MODE_RDONLY, MPI_INFO_NULL, &input));
    check(MPI_File_get_size(input, &bytes));
    require(bytes > 2 * 65536 && bytes <= INT_MAX, "expected a finite multi-stripe file");
    MPI_Offset start = bytes * rank / ranks;
    int count = (int)(bytes * (rank + 1) / ranks - start);
    unsigned char *expected = malloc((size_t)count);
    unsigned char *observed = malloc((size_t)count);
    require(expected != NULL && observed != NULL, "rank buffer allocation failed");
    check(MPI_File_read_at_all(input, start, expected, count, MPI_BYTE, &status));
    check_count(&status, count);
    check(MPI_File_close(&input));

    int mode = MPI_MODE_CREATE | MPI_MODE_EXCL | MPI_MODE_RDWR;
    check(MPI_File_open(MPI_COMM_WORLD, argv[2], mode, MPI_INFO_NULL, &output));
    check(MPI_File_write_at_all(output, start, expected, count, MPI_BYTE, &status));
    check_count(&status, count);
    check(MPI_File_sync(output));
    check(MPI_File_close(&output));
    check(MPI_File_open(MPI_COMM_WORLD, argv[2], MPI_MODE_RDONLY, MPI_INFO_NULL, &output));
    check(MPI_File_read_at_all(output, start, observed, count, MPI_BYTE, &status));
    check_count(&status, count);
    require(memcmp(expected, observed, (size_t)count) == 0, "rank byte parity failed");
    check(MPI_File_close(&output));
    printf("rank %d/%d read, wrote and reread %d bytes at %lld\n",
           rank, ranks, count, (long long)start);
    free(observed);
    free(expected);
    check(MPI_Finalize());
    return 0;
}
